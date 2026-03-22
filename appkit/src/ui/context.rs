use objc2::rc::Retained;
use objc2_app_kit::{NSStackView, NSView};
use reactive_core::ContextKey;

#[derive(Clone)]
pub(super) enum ViewParent {
    Window(Retained<NSView>),
    Stack(Retained<NSStackView>),
}

impl ViewParent {
    pub(super) fn add_child(&self, child: Retained<NSView>) {
        match self {
            ViewParent::Window(parent) => {
                child.setFrame(parent.bounds());
                parent.addSubview(&child);
            }
            ViewParent::Stack(stack) => {
                stack.addArrangedSubview(&child);
            }
        }
    }
}

pub(super) static PARENT_VIEW: ContextKey<ViewParent> = ContextKey::new();
