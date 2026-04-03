use reactive_core::{Component, ReadStoredSignal, SetupContext, Signal};
use ui_core::widgets::{List, ListData, ListOrientation};

/// GTK list view.
///
/// A full implementation using `gtk::ListView` + `gio::ListStore` requires
/// custom GObject item types and is deferred.  The current stub panics at
/// runtime if instantiated; the demo application does not use `List`.
pub struct ListView;

impl Component for ListView {
    fn setup(self: Box<Self>, _ctx: &mut SetupContext) {
        unimplemented!("GtkListView is not yet implemented")
    }
}

impl List for ListView {
    fn new<L, I, C>(
        _list_data: impl Signal<Value = L> + 'static,
        _component_factory: impl FnMut(ReadStoredSignal<I>) -> C + 'static,
    ) -> Self
    where
        L: ListData<I> + 'static,
        C: Component + 'static,
        I: 'static,
    {
        ListView
    }

    fn orientation(self, _orientation: ListOrientation) -> Self {
        self
    }
}
