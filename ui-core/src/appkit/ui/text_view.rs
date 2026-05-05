use crate::widgets::{Modifier, NativeView, PlatformTextType, TextCommand, WithModifier};
use derive_more::Display;
use futures::channel::mpsc::Receiver;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{define_class, msg_send, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSTextDelegate, NSTextView, NSTextViewDelegate};
use objc2_foundation::{
    NSMutableCopying, NSNotification, NSObject, NSObjectProtocol, NSRange, NSString,
};
use reactive_core::{Component, SetupContext, Signal};
use std::cell::RefCell;
use std::ops::Range;

#[derive(Default)]
pub struct TextInput {
    on_text_changed: Option<Box<dyn FnMut(&NSString)>>,
    on_selection_changed: Option<Box<dyn FnMut(Range<usize>)>>,
    commander: Option<Receiver<TextCommand<AppkitString>>>,
    font_size: Option<Box<dyn Signal<Value = f64>>>,
    modifier: Modifier,
}

pub type TextView = TextInput;

struct TextViewDelegateIvars {
    on_text_changed: RefCell<Option<Box<dyn FnMut(&NSString)>>>,
    on_selection_changed: RefCell<Option<Box<dyn FnMut(Range<usize>)>>>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = TextViewDelegateIvars]
    #[name = "ReactiveTextViewDelegate"]
    struct TextViewDelegate;

    unsafe impl NSObjectProtocol for TextViewDelegate {}

    unsafe impl NSTextDelegate for TextViewDelegate {
        #[unsafe(method(textDidChange:))]
        fn text_did_change(&self, notification: &NSNotification) {
            let Some(text_view) = notification
                .object()
                .and_then(|object| object.downcast::<NSTextView>().ok())
            else {
                return;
            };

            let string = text_view.string();
            if let Some(on_text_changed) = &mut *self.ivars().on_text_changed.borrow_mut() {
                on_text_changed(&string);
            }
        }
    }

    unsafe impl NSTextViewDelegate for TextViewDelegate {
        #[unsafe(method(textViewDidChangeSelection:))]
        fn text_view_did_change_selection(&self, notification: &NSNotification) {
            let Some(text_view) = notification
                .object()
                .and_then(|object| object.downcast::<NSTextView>().ok())
            else {
                return;
            };

            let selected_range = text_view.selectedRange();
            if let Some(on_selection_changed) = &mut *self.ivars().on_selection_changed.borrow_mut()
            {
                on_selection_changed(
                    selected_range.location..selected_range.location + selected_range.length,
                );
            }
        }
    }
);

impl TextViewDelegate {
    fn new(
        mtm: MainThreadMarker,
        on_text_changed: Option<Box<dyn FnMut(&NSString)>>,
        on_selection_changed: Option<Box<dyn FnMut(Range<usize>)>>,
    ) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(TextViewDelegateIvars {
            on_text_changed: RefCell::new(on_text_changed),
            on_selection_changed: RefCell::new(on_selection_changed),
        });
        unsafe { msg_send![super(this), init] }
    }
}

#[derive(Clone, Display, PartialEq, Eq, Hash)]
pub struct AppkitString(Retained<NSString>);

impl<'a> From<&'a str> for AppkitString {
    fn from(value: &'a str) -> Self {
        Self(NSString::from_str(value))
    }
}

unsafe impl Send for AppkitString {}
unsafe impl Sync for AppkitString {}

impl PlatformTextType for AppkitString {
    type RefType<'a> = &'a NSString;

    fn len(&self) -> usize {
        self.0.len()
    }

    fn replace(&self, range: Range<usize>, with: &Self::RefType<'_>) -> Self {
        let s = self.0.mutableCopy();
        s.replaceCharactersInRange_withString(NSRange::new(range.start, range.end), with);
        Self(s.into_super())
    }

    fn as_str(&self) -> Option<&str> {
        None
    }
}

impl Component for TextInput {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let Self {
            on_text_changed,
            on_selection_changed,
            commander: _,
            font_size: _,
            modifier,
        } = *self;

        let text_view = NativeView::new(
            |_| NSTextView::new(MainThreadMarker::new().unwrap()),
            |v| v.into_super().into_super(),
            |_, _| {},
            modifier,
            &super::VIEW_REGISTRY_KEY,
        )
        .setup_in_component(ctx);

        if on_text_changed.is_some() || on_selection_changed.is_some() {
            let delegate = TextViewDelegate::new(
                MainThreadMarker::new().expect("must be on main thread"),
                on_text_changed,
                on_selection_changed,
            );
            text_view.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
            ctx.on_cleanup(move || {
                text_view.setDelegate(None);
                drop(delegate);
            });
        }
    }
}

impl WithModifier for TextInput {
    fn modifier(mut self, modifier: Modifier) -> Self {
        self.modifier = modifier;
        self
    }
}

impl crate::widgets::TextInput for TextInput {
    type PlatformTextType = AppkitString;

    fn new() -> Self {
        Self::default()
    }

    fn with_commander(mut self, rx: Receiver<TextCommand<Self::PlatformTextType>>) -> Self {
        self.commander.replace(rx);
        self
    }

    fn with_on_text_changed(
        mut self,
        on_change: impl FnMut(<Self::PlatformTextType as PlatformTextType>::RefType<'_>) + 'static,
    ) -> Self {
        self.on_text_changed.replace(Box::new(on_change));
        self
    }

    fn with_on_selection_changed(
        mut self,
        on_selection_changed: impl FnMut(Range<usize>) + 'static,
    ) -> Self {
        self.on_selection_changed
            .replace(Box::new(on_selection_changed));
        self
    }

    fn font_size(mut self, size: impl Signal<Value = f64> + 'static) -> Self {
        self.font_size.replace(Box::new(size));
        self
    }
}
