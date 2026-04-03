use objc2::rc::Retained;
use objc2_app_kit::NSView;
use reactive_core::{ContextKey, StoredSignal};
use ui_core::ChildEntry;

pub type ChildViewEntry = ChildEntry<Retained<NSView>>;

pub(crate) static CHILD_VIEW: ContextKey<StoredSignal<Option<ChildViewEntry>>> = ContextKey::new();

pub(crate) static CHILDREN_VIEWS: ContextKey<Vec<StoredSignal<Option<ChildViewEntry>>>> =
    ContextKey::new();
