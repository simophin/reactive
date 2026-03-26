use std::cell::RefCell;
use std::ops::Range;

use apple::Prop;
use apple::bindable::BindableView;
use objc2::rc::Retained;
use objc2::{DefinedClass, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::*;
use objc2_foundation::*;
use reactive_core::{Signal, SignalExt};

use super::view_component::AppKitViewComponent;

pub type TextView = AppKitViewComponent<NSTextView, ()>;

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
// ReactiveTextView — NSTextView subclass with true interception points.
//
// Text changes are intercepted via `shouldChangeTextInRange:replacementString:`
// which fires BEFORE the mutation. We construct the proposed text, call the
// callback, and return whether the signal accepted the change — naturally
// breaking the loop without any guard variable.
//
// Selection changes are intercepted via `setSelectedRange:affinity:stillSelecting:`,
// which all selection changes funnel through.
// No notification center observers are used.
// ---------------------------------------------------------------------------

struct ReactiveTextViewState {
    /// Returns `true` to approve the proposed text (let NSTextView apply it),
    /// or `false` to reject it (the effect will correct the view next tick).
    on_text_change: RefCell<Box<dyn FnMut(Retained<NSString>) -> bool>>,
    /// Called with the proposed selection; returns the selection the signal
    /// settled on (may differ if the callback transformed it). NSTextView is
    /// then instructed to apply the returned range — breaking the loop without
    /// any guard variable, because we call `super` directly.
    on_selection_change: RefCell<Box<dyn FnMut(Range<usize>) -> Range<usize>>>,
}

define_class!(
    #[unsafe(super(NSTextView))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ReactiveTextViewState]
    #[name = "ReactiveTextView"]
    struct ReactiveTextView;

    unsafe impl NSObjectProtocol for ReactiveTextView {}

    impl ReactiveTextView {
        /// Called BEFORE any text mutation. We construct the proposed string,
        /// call `on_text_change` (which updates the signal), then compare the
        /// signal's new value to the proposed text:
        ///
        /// - Equal → return `true` — NSTextView applies the change as-is.
        /// - Different → return `false` — NSTextView discards the change; the
        ///   reactive effect will push the signal's (transformed) value next tick.
        #[unsafe(method(shouldChangeTextInRange:replacementString:))]
        fn should_change_text_in_range(
            &self,
            range: NSRange,
            replacement: Option<&NSString>,
        ) -> objc2::runtime::Bool {
            // Standard checks: isEditable, delegate, etc.
            let allowed: bool = unsafe {
                msg_send![super(self), shouldChangeTextInRange: range, replacementString: replacement]
            };
            if !allowed {
                return objc2::runtime::Bool::NO;
            }
            let Some(replacement) = replacement else {
                // Attribute-only change (spans, IME marks) — always approve.
                return objc2::runtime::Bool::YES;
            };
            // Build proposed string by mutably copying the current content and
            // applying the replacement in-place. This stays entirely in UTF-16
            // (no Rust String allocations or encoding conversions).
            let proposed = self.string().mutableCopy();
            proposed.replaceCharactersInRange_withString(range, replacement);
            let accepted = self.ivars().on_text_change.borrow_mut()(Retained::into_super(proposed));
            objc2::runtime::Bool::new(accepted)
        }

        /// All selection changes funnel through this method — user cursor
        /// moves, drag selections, and programmatic setSelectedRange: calls.
        ///
        /// During a drag (`still_selecting = true`) we let NSTextView apply the
        /// visual rubber-band selection immediately without notifying the signal,
        /// since intermediate drag positions are noise.
        ///
        /// When the selection settles (`still_selecting = false`) we call the
        /// callback with the proposed range and apply whatever range the signal
        /// returns. Calling `super` directly bypasses this override, so there
        /// is no recursion risk even if the signal transforms the range.
        #[unsafe(method(setSelectedRange:affinity:stillSelecting:))]
        fn set_selected_range_affinity_still_selecting(
            &self,
            range: NSRange,
            affinity: NSSelectionAffinity,
            still_selecting: bool,
        ) {
            if still_selecting {
                // Just track visual feedback during the drag; no signal update.
                let _: () = unsafe {
                    msg_send![
                        super(self),
                        setSelectedRange: range,
                        affinity: affinity,
                        stillSelecting: true
                    ]
                };
                return;
            }
            let proposed = range.location..range.location + range.length;
            let accepted = (self.ivars().on_selection_change.borrow_mut())(proposed);
            let accepted_range = NSRange {
                location: accepted.start,
                length: accepted.len(),
            };
            let _: () = unsafe {
                msg_send![
                    super(self),
                    setSelectedRange: accepted_range,
                    affinity: affinity,
                    stillSelecting: false
                ]
            };
        }
    }
);

impl ReactiveTextView {
    fn new(
        on_text_change: impl FnMut(Retained<NSString>) -> bool + 'static,
        on_selection_change: impl FnMut(Range<usize>) -> Range<usize> + 'static,
        mtm: MainThreadMarker,
    ) -> Retained<Self> {
        let state = ReactiveTextViewState {
            on_text_change: RefCell::new(Box::new(on_text_change)),
            on_selection_change: RefCell::new(Box::new(on_selection_change)),
        };
        let this = Self::alloc(mtm).set_ivars(state);
        unsafe { msg_send![super(this), initWithFrame: NSRect::ZERO] }
    }
}

fn make_reactive_text_view(
    on_text_change: impl FnMut(Retained<NSString>) -> bool + 'static,
    on_selection_change: impl FnMut(Range<usize>) -> Range<usize> + 'static,
    mtm: MainThreadMarker,
) -> Retained<NSTextView> {
    ReactiveTextView::new(on_text_change, on_selection_change, mtm).into_super()
}

fn into_nsview(view: Retained<NSTextView>) -> Retained<NSView> {
    view.into_super().into_super()
}

// ---------------------------------------------------------------------------
// TextView factories
// ---------------------------------------------------------------------------

impl TextView {
    /// A non-editable, non-selectable text label backed by NSTextView.
    pub fn label(text: impl Signal<Value = String> + 'static) -> Self {
        AppKitViewComponent::create(
            |_| {
                let mtm = MainThreadMarker::new().expect("must be on main thread");
                let tv = NSTextView::initWithFrame(NSTextView::alloc(mtm), NSRect::ZERO);
                tv.setEditable(false);
                tv.setDrawsBackground(false);
                tv
            },
            into_nsview,
        )
        .bind(PROP_STRING, text.map_value(|s| NSString::from_str(&s)))
    }

    /// A fully reactive text input backed by ReactiveTextStorage (backing
    /// store) and ReactiveTextView (interception).
    ///
    /// - `text` / `on_text_change`: the string is the source of truth;
    ///   `on_text_change` fires on every user edit (before layout).
    /// - `selection` / `on_selection_change`: UTF-16 code unit range, maps
    ///   directly to NSRange; `on_selection_change` fires when the selection
    ///   settles (not on every drag frame).
    pub fn input(
        text: impl Signal<Value = Retained<NSString>> + Clone + 'static,
        on_text_change: impl for<'a> FnMut(&'a Retained<NSString>) + 'static,
        selection: impl Signal<Value = Range<usize>> + Clone + 'static,
        on_selection_change: impl FnMut(Range<usize>) + 'static,
    ) -> Self {
        let text_for_check = text.clone();
        let mut on_text_change = on_text_change;
        let text_change_cb = move |proposed: Retained<NSString>| -> bool {
            on_text_change(&proposed);
            text_for_check.read().isEqualToString(&proposed)
        };
        let selection_for_check = selection.clone();
        let mut on_selection_change = on_selection_change;
        let selection_change_cb = move |proposed: Range<usize>| -> Range<usize> {
            on_selection_change(proposed.clone());
            selection_for_check.read()
        };
        AppKitViewComponent::create(
            move |_| {
                let mtm = MainThreadMarker::new().expect("must be on main thread");
                let tv = make_reactive_text_view(text_change_cb, selection_change_cb, mtm);
                tv.setEditable(true);
                tv
            },
            into_nsview,
        )
        .bind(PROP_STRING, text)
        .bind(
            PROP_SELECTEDRANGE,
            selection.map_value(|r| NSRange {
                location: r.start,
                length: r.len(),
            }),
        )
    }

    /// Convenience wrapper for callers that only care about the plain string
    /// and not selection. Selection changes from the view are ignored.
    pub fn input_text(
        text: impl Signal<Value = String> + Clone + 'static,
        on_change: impl for<'a> FnMut(&'a str) + 'static,
    ) -> Self {
        let text_for_check = text.clone();
        let mut on_change = on_change;
        let text_change_cb = move |proposed: Retained<NSString>| -> bool {
            let proposed_str = proposed.to_string();
            on_change(&proposed.to_string());
            text_for_check.read() == proposed_str
        };
        AppKitViewComponent::create(
            move |_| {
                let mtm = MainThreadMarker::new().expect("must be on main thread");
                let tv = make_reactive_text_view(text_change_cb, |r| r, mtm);
                tv.setEditable(true);
                tv
            },
            into_nsview,
        )
        .bind(PROP_STRING, text.map_value(|s| NSString::from_str(&s)))
    }
}
