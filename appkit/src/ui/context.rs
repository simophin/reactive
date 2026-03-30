use objc2::rc::Retained;
use objc2_app_kit::NSView;
use reactive_core::{ContextKey, StoredSignal};
use ui_core::layout::ChildLayoutInfo;

#[derive(Clone, PartialEq, Eq)]
pub struct ChildViewEntry {
    pub view: Retained<NSView>,
    pub layout: ChildLayoutInfo,
}

pub(crate) static CHILD_VIEW: ContextKey<StoredSignal<Option<ChildViewEntry>>> = ContextKey::new();

pub(crate) static CHILDREN_VIEWS: ContextKey<Vec<StoredSignal<Option<ChildViewEntry>>>> =
    ContextKey::new();
