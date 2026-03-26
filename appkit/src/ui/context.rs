use objc2::rc::Retained;
use objc2_app_kit::{NSStackView, NSView};
use reactive_core::ContextKey;

pub(super) static PARENT_VIEW: ContextKey<ViewParent> = ContextKey::new();

#[derive(Clone)]
pub(super) enum ViewParent {
    View(Retained<NSView>),
    Stack(Retained<NSStackView>),
}

impl ViewParent {
    /// Add `child` as a subview (or arranged subview for NSStackView).
    pub fn add_child(&self, child: Retained<NSView>) {
        match self {
            Self::View(v) => v.addSubview(&child),
            Self::Stack(s) => s.addArrangedSubview(&child),
        }
    }

    /// Remove `child` from its superview.
    pub fn remove_child(&self, child: &NSView) {
        child.removeFromSuperview();
    }
}
