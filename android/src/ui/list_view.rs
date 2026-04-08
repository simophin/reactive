use ui_core::widgets::{List, ListData, ListComparator, ListOrientation};
use reactive_core::{BoxedComponent, Component, SetupContext, ReadStoredSignal};
use crate::ui::view_component::{AndroidViewBuilder, AndroidViewComponent};

pub struct AndroidListView;

impl List for AndroidListView {
    fn new_with_comparator<L, I, C, Comp>(
        list_data: impl reactive_core::Signal<Value = L> + 'static,
        mut component_factory: impl FnMut(ReadStoredSignal<I>) -> C + 'static,
        list_comparator: Comp,
    ) -> Self
    where
        L: ListData<I> + 'static,
        C: Component + 'static,
        I: Clone + 'static,
        Comp: ListComparator<I> + 'static,
    {
        AndroidViewComponent(
            AndroidViewBuilder::create_no_child(
                |_ctx| {
                    todo!("Create RecyclerView via JNI")
                },
                |w| todo!("Convert to AndroidView"),
            )
        )
    }

    fn orientation(self, orientation: ListOrientation) -> Self {
        self
    }
}
