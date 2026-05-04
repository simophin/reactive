use reactive_core::{Component, ReadStoredSignal, SetupContext, Signal};
use ui_core::widgets::{List, ListComparator, ListData, ListOrientation};

use crate::android::ui::flex::AndroidFlex;

pub struct AndroidListView {
    orientation: ListOrientation,
    setup_fn: Box<dyn FnOnce(ListOrientation, &mut SetupContext)>,
}

impl List for AndroidListView {
    fn new_with_comparator<L, I, C, Comp>(
        list_data: impl Signal<Value = L> + 'static,
        mut component_factory: impl FnMut(ReadStoredSignal<I>) -> C + 'static,
        _list_comparator: Comp,
    ) -> Self
    where
        L: ListData<I> + 'static,
        C: Component + 'static,
        I: Clone + 'static,
        Comp: ListComparator<I> + 'static,
    {
        AndroidListView {
            orientation: ListOrientation::Vertical,
            setup_fn: Box::new(move |orientation, ctx| {
                setup_list(ctx, orientation, list_data, &mut component_factory);
            }),
        }
    }

    fn orientation(mut self, orientation: ListOrientation) -> Self {
        self.orientation = orientation;
        self
    }
}

impl Component for AndroidListView {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        (self.setup_fn)(self.orientation, ctx);
    }
}

fn setup_list<I, L, C>(
    ctx: &mut SetupContext,
    orientation: ListOrientation,
    list_data: impl Signal<Value = L> + 'static,
    component_factory: &mut impl FnMut(ReadStoredSignal<I>) -> C,
) where
    I: Clone + 'static,
    L: ListData<I> + 'static,
    C: Component + 'static,
{
    let data = list_data.read();
    let flex = if matches!(orientation, ListOrientation::Vertical) {
        let mut column = <AndroidFlex as ui_core::widgets::Column>::new();
        for index in 0..data.count() {
            let item = data.get_item(index).cloned().expect("list item in bounds");
            let signal = ctx.create_signal(item);
            column = <AndroidFlex as ui_core::widgets::Column>::child(
                column,
                component_factory(signal.read_only()),
            );
        }
        column
    } else {
        let mut row = <AndroidFlex as ui_core::widgets::Row>::new();
        for index in 0..data.count() {
            let item = data.get_item(index).cloned().expect("list item in bounds");
            let signal = ctx.create_signal(item);
            row = <AndroidFlex as ui_core::widgets::Row>::child(
                row,
                component_factory(signal.read_only()),
            );
        }
        row
    };

    Box::new(flex).setup(ctx);
    ctx.create_effect(move |_, _| {
        let _ = list_data.read();
    });
}
