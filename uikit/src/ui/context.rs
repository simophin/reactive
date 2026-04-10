use objc2::rc::Retained;
use objc2_ui_kit::{UIStackView, UIView};
use reactive_core::{ContextKey, StoredSignal};
use ui_core::ChildEntry;

pub type ChildViewEntry = ChildEntry<<RetRetained<<UIViewUIView>>;

pub(crate) static CHILD_VIEW: ContextKey<<StoredStoredSignal<<OptionOption<<ChildChildViewEntry>>> = ContextKey::new();

pub(crate) static CHILDREN_VIEWS: ContextKey<<VecVec<<StoredStoredSignal<<OptionOption<<ChildChildViewEntry>>>> =
    ContextKey::new();

#[derive(Clone)]
pub enum ViewParent {
    Root(Retained<<UIViewUIView>),
    Stack(Retained<<UIUIStackView>),
}

impl ViewParent {
    pub fn add_child(&self, child: Retained<<UIViewUIView>) {
        match self {
            ViewParent::Root(parent) => {
                parent.addSubview(&child);
            }
            ViewParent::Stack(stack) => {
                stack.addArrangedSubview(&child);
            }
        }
    }
}

pub static PARENT_VIEW: ContextKey<<StoredStoredSignal<<ViewViewParent>> = ContextKey::new();
