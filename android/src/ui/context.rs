use reactive_core::ContextKey;
use ui_core::ChildEntry;
use crate::ui::view_component::AndroidView;

pub type ChildViewEntry = ChildEntry<AndroidView>;

pub(crate) static CHILD_VIEW: ContextKey<reactive_core::StoredSignal<Option<ChildViewEntry>>> =
    ContextKey::new();

pub(crate) static CHILDREN_VIEWS: ContextKey<Vec<reactive_core::StoredSignal<Option<ChildViewEntry>>>> =
    ContextKey::new();
