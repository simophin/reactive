use objc2::rc::Retained;
use objc2_app_kit::NSView;
use reactive_core::{ContextKey, Signal, StoredSignal};
use std::rc::Rc;
use ui_core::layout::LayoutHints;

#[derive(PartialEq, Eq)]
pub struct ChildViewEntry {
    pub view: Retained<NSView>,
    pub layout_hints: LayoutHints,
}

pub(crate) static CHILD_VIEW: ContextKey<StoredSignal<Option<ChildViewEntry>>> = ContextKey::new();

pub(crate) static CHILDREN_VIEWS: ContextKey<Vec<StoredSignal<Option<ChildViewEntry>>>> =
    ContextKey::new();
