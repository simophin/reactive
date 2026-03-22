use objc2::rc::Retained;
use objc2_ui_kit::{UIStackView, UIView};
use reactive_core::ContextKey;

#[derive(Clone)]
pub(super) enum ViewParent {
    Window(Retained<UIView>),
    Stack(Retained<UIStackView>),
}

impl ViewParent {
    pub(super) fn add_child(&self, child: Retained<UIView>) {
        match self {
            ViewParent::Window(parent) => {
                parent.addSubview(&child);
            }
            ViewParent::Stack(stack) => {
                stack.addArrangedSubview(&child);
            }
        }
    }
}

pub(super) static PARENT_VIEW: ContextKey<ViewParent> = ContextKey::new();
