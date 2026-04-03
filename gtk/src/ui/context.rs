use reactive_core::{ContextKey, StoredSignal};
use ui_core::ChildEntry;

pub type ChildWidgetEntry = ChildEntry<gtk4::Widget>;

pub(crate) static CHILD_WIDGET: ContextKey<StoredSignal<Option<ChildWidgetEntry>>> =
    ContextKey::new();

pub(crate) static CHILDREN_WIDGETS: ContextKey<Vec<StoredSignal<Option<ChildWidgetEntry>>>> =
    ContextKey::new();
