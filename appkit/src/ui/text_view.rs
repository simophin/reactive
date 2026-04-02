use super::view_component::AppKitViewBuilder;
use crate::view_component::{AppKitViewComponent, NoChildView};
use apple::Prop;
use derive_more::Display;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{DefinedClass, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::*;
use objc2_foundation::*;
use reactive_core::Signal;
use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;
use ui_core::widgets::{PlatformTextType, TextChange, TextInputState};

pub type TextView = AppKitViewComponent<NSTextView, NoChildView>;

apple::view_props! {
    TextView on NSTextView {
        pub selectable: bool;
        selectedRange: NSRange;
    }
}

pub static PROP_FONT_SIZE: &Prop<TextView, NSTextView, f64> = &Prop::new(|view, size| {
    let font = NSFont::systemFontOfSize(size);
    view.setFont(Some(&font));
});

static PROP_STRING: &Prop<TextView, NSTextView, Retained<NSString>> = &Prop::new(|view, value| {
    view.setString(&value);
});

// ---------------------------------------------------------------------------
// ReactiveTextStorage — NSTextStorage subclass.
//
// Acts as a pure backing store. The only job of replaceCharactersInRange: is
// to mutate the string and post the edit notification so NSTextKit redraws.
// It does NOT call any application callback — that is ReactiveTextView's job.
//
// set_text() is the programmatic path: wraps the mutation in
// beginEditing/endEditing so it never goes through shouldChangeTextInRange:.
// ---------------------------------------------------------------------------

struct TextStorageState {
    text: Retained<NSMutableString>,
    empty_attrs: Retained<NSDictionary<NSString, AnyObject>>,
}

define_class!(
    #[unsafe(super(NSTextStorage))]
    #[thread_kind = MainThreadOnly]
    #[ivars = TextStorageState]
    #[name = "ReactiveTextStorage"]
    struct TextStorage;

    unsafe impl NSObjectProtocol for TextStorage {}

    impl TextStorage {
        #[unsafe(method(string))]
        fn string(&self) -> &NSString {
            &self.ivars().text
        }

        #[unsafe(method(attributesAtIndex:effectiveRange:))]
        fn attributes_at_index(
            &self,
            _location: NSUInteger,
            range: *mut NSRange,
        ) -> &NSDictionary<NSString, AnyObject> {
            if !range.is_null() {
                unsafe {
                    *range = NSRange {
                        location: 0,
                        length: self.ivars().text.length(),
                    };
                }
            }
            &self.ivars().empty_attrs
        }

        #[unsafe(method(replaceCharactersInRange:withString:))]
        fn replace_characters_in_range_with_string(&self, range: NSRange, replacement: &NSString) {
            let old_length = self.ivars().text.length();
            self.ivars()
                .text
                .replaceCharactersInRange_withString(range, replacement);
            let delta = self.ivars().text.length() as isize - old_length as isize;
            self.edited_range_changeInLength(
                NSTextStorageEditActions::EditedCharacters,
                range,
                delta,
            );
        }

        #[unsafe(method(setAttributes:range:))]
        fn set_attributes_range(
            &self,
            _attrs: Option<&NSDictionary<NSString, AnyObject>>,
            range: NSRange,
        ) {
            self.edited_range_changeInLength(
                NSTextStorageEditActions::EditedAttributes,
                range,
                0,
            );
        }
    }
);

impl TextStorage {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let state = TextStorageState {
            text: NSMutableString::new(),
            empty_attrs: NSDictionary::new(),
        };
        let this = Self::alloc(mtm).set_ivars(state);
        unsafe { msg_send![super(this), init] }
    }

    /// Programmatic update from a signal. Wraps in beginEditing/endEditing so
    /// NSTextKit redraws without going through shouldChangeTextInRange:.
    fn set_text(&self, new_text: &NSString) {
        self.beginEditing();
        let old_length = self.ivars().text.length();
        let new_length = new_text.length();
        let full_range = NSRange {
            location: 0,
            length: old_length,
        };
        self.ivars()
            .text
            .replaceCharactersInRange_withString(full_range, new_text);
        self.edited_range_changeInLength(
            NSTextStorageEditActions::EditedCharacters,
            full_range,
            new_length as isize - old_length as isize,
        );
        self.endEditing();
    }
}

// ---------------------------------------------------------------------------
// ReactiveTextView — NSTextView subclass.
//
// Intercepts shouldChangeTextInRange:replacementString: which fires BEFORE
// NSTextKit applies the mutation. We call on_change here:
//
// - If the caller updates the signal (accepts the change) we return YES and
//   NSTextKit applies it to the TextStorage.
// - If the caller does NOT update the signal (rejects the change) we return
//   NO. NSTextKit discards the change with no mutation at all — no flicker.
//
// The programmatic path (signal → set_text) mutates TextStorage directly and
// never passes through this method, so there is no recursion risk.
// ---------------------------------------------------------------------------

struct ReactiveTextViewState {
    on_change: RefCell<Box<dyn FnMut(NSRange, &NSString) -> bool>>,
}

define_class!(
    #[unsafe(super(NSTextView))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ReactiveTextViewState]
    #[name = "ReactiveTextView"]
    struct ReactiveTextView;

    unsafe impl NSObjectProtocol for ReactiveTextView {}

    impl ReactiveTextView {
        /// Returns YES to let NSTextKit apply the change, NO to discard it.
        #[unsafe(method(shouldChangeTextInRange:replacementString:))]
        fn should_change_text_in_range(
            &self,
            range: NSRange,
            replacement: Option<&NSString>,
        ) -> objc2::runtime::Bool {
            let allowed: bool = unsafe {
                msg_send![super(self), shouldChangeTextInRange: range, replacementString: replacement]
            };
            if !allowed {
                return objc2::runtime::Bool::NO;
            }
            match replacement {
                Some(replacement) => {
                    let accepted = self.ivars().on_change.borrow_mut()(range, replacement);
                    objc2::runtime::Bool::from(accepted)
                }
                None => objc2::runtime::Bool::YES,
            }
        }
    }
);

impl ReactiveTextView {
    fn new(
        on_change: impl FnMut(NSRange, &NSString) -> bool + 'static,
        mtm: MainThreadMarker,
    ) -> Retained<Self> {
        let state = ReactiveTextViewState {
            on_change: RefCell::new(Box::new(on_change)),
        };
        let this = Self::alloc(mtm).set_ivars(state);
        unsafe { msg_send![super(this), initWithFrame: NSRect::ZERO] }
    }
}

// ---------------------------------------------------------------------------
// AppKitText — the platform text type for AppKit text inputs.
// ---------------------------------------------------------------------------

#[derive(Clone, Display, PartialEq, Eq)]
pub struct AppKitText(Retained<NSString>);

impl<'a> From<&'a str> for AppKitText {
    fn from(value: &'a str) -> Self {
        Self(NSString::from_str(value))
    }
}
unsafe impl Sync for AppKitText {}
unsafe impl Send for AppKitText {}

impl PlatformTextType for AppKitText {
    type RefType<'a> = &'a NSString;

    fn len(&self) -> usize {
        self.0.len()
    }

    fn replace(&self, range: Range<usize>, with: &Self::RefType<'_>) -> Self {
        let new_string = self.0.mutableCopy();
        new_string.replaceCharactersInRange_withString(
            NSRange {
                location: range.start,
                length: range.len(),
            },
            *with,
        );
        Self(new_string.into_super())
    }

    fn as_str(&self) -> Option<&str> {
        None
    }
}

// ---------------------------------------------------------------------------
// TextInput trait implementation
// ---------------------------------------------------------------------------

impl ui_core::widgets::TextInput for TextView {
    type PlatformTextType = AppKitText;

    fn new(
        value: impl Signal<Value = TextInputState<Self::PlatformTextType>> + 'static,
        on_change: impl for<'a> FnMut(
            TextChange<<Self::PlatformTextType as PlatformTextType>::RefType<'a>>,
        ) + 'static,
    ) -> Self {
        let on_change = Rc::new(RefCell::new(on_change));
        Self(AppKitViewBuilder::create_no_child(
            move |ctx| {
                let mtm = MainThreadMarker::new().unwrap();
                let on_change = on_change.clone();

                // Box the signal so it can be shared between the callback and
                // the effect without requiring Signal: Clone.
                let value: Rc<dyn Signal<Value = TextInputState<AppKitText>>> = Rc::new(value);
                let value_for_effect = value.clone();

                let storage = TextStorage::new(mtm);

                let layout_manager = NSLayoutManager::new();
                let container = NSTextContainer::initWithSize(
                    mtm.alloc::<NSTextContainer>(),
                    NSSize {
                        width: f64::MAX,
                        height: f64::MAX,
                    },
                );
                layout_manager.addTextContainer(&container);
                storage.addLayoutManager(&layout_manager);

                let text_view = ReactiveTextView::new(
                    move |range, replacement| {
                        let initial = value.read().text.clone();
                        on_change.borrow_mut()(TextChange::Replacement {
                            replace: range.location..range.location + range.length,
                            with: replacement,
                        });
                        // Return true (YES) if the caller updated the signal,
                        // meaning the change is accepted as-is.
                        value.read().text != initial
                    },
                    mtm,
                );

                // Connect ReactiveTextView to our custom storage by replacing
                // the default text container with ours.
                unsafe { text_view.setTextContainer(Some(&container)) };

                // Effect: signal → storage (programmatic path).
                let storage = storage.clone();
                ctx.create_effect(move |_, _| {
                    storage.set_text(&value_for_effect.read().text.0);
                });

                text_view.into_super()
            },
            |t| t.into_super().into_super(),
        ))
    }

    fn font_size(self, size: impl Signal<Value = f64> + 'static) -> Self {
        Self(self.0.bind(PROP_FONT_SIZE, size))
    }
}
