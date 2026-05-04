use std::cell::RefCell;
use std::rc::Rc;

use crate::context::{CHILD_WIDGET, ChildWidgetEntry};
use crate::gtk::ui::flex::Flex;
use glib::object::ObjectExt;
use gtk4::prelude::*;
use gtk4::{
    ListView as GtkListView, NoSelection, Orientation as GtkOrientation, ScrolledWindow, gio,
};
use reactive_core::{
    BoxedComponent, Component, IntoSignal, ReadStoredSignal, SetupContext, Signal, StoredSignal,
};
use ui_core::layout::{BOX_MODIFIERS, ChildLayoutInfo, CrossAxisAlignment, FLEX_PARENT_DATA};
use ui_core::widgets::{Column, DiffOp, List, ListComparator, ListData, ListOrientation, diff};

const ROW_STATE_KEY: &str = "reactive-gtk-list-row-state";

struct RowState<I> {
    item_signal: StoredSignal<I>,
    component_id: reactive_core::ComponentId,
    widget: gtk4::Widget,
}

pub struct ListView {
    orientation: ListOrientation,
    setup_fn: Box<dyn FnOnce(ListOrientation, &mut SetupContext)>,
}

impl Component for ListView {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        (self.setup_fn)(self.orientation, ctx)
    }
}

impl List for ListView {
    fn new_with_comparator<L, I, C, Comp>(
        list_data: impl Signal<Value = L> + 'static,
        mut component_factory: impl FnMut(ReadStoredSignal<I>) -> C + 'static,
        list_comparator: Comp,
    ) -> Self
    where
        L: ListData<I> + 'static,
        C: Component + 'static,
        I: Clone + 'static,
        Comp: ListComparator<I> + 'static,
    {
        let factory: Rc<RefCell<dyn FnMut(ReadStoredSignal<I>) -> BoxedComponent>> =
            Rc::new(RefCell::new(move |signal| -> BoxedComponent {
                Box::new(component_factory(signal))
            }));
        let comparator = Rc::new(list_comparator);

        Self {
            orientation: ListOrientation::Vertical,
            setup_fn: Box::new(move |orientation, ctx| {
                setup_list(ctx, orientation, list_data, factory, comparator);
            }),
        }
    }

    fn orientation(mut self, orientation: ListOrientation) -> Self {
        self.orientation = orientation;
        self
    }
}

fn setup_list<I, L, Comp>(
    ctx: &mut SetupContext,
    orientation: ListOrientation,
    list_data: impl Signal<Value = L> + 'static,
    factory: Rc<RefCell<dyn FnMut(ReadStoredSignal<I>) -> BoxedComponent>>,
    comparator: Rc<Comp>,
) where
    I: Clone + 'static,
    L: ListData<I> + 'static,
    Comp: ListComparator<I> + 'static,
{
    let scope = ctx.scope();
    let parent_id = ctx.component_id();
    let store = gio::ListStore::new::<glib::BoxedAnyObject>();

    let item_factory = gtk4::SignalListItemFactory::new();

    item_factory.connect_bind({
        let scope = scope.clone();
        let factory = Rc::clone(&factory);
        let comparator = Rc::clone(&comparator);

        move |_, list_item| {
            let list_item = as_list_item(list_item, "bind");
            let item_object = list_item
                .item()
                .and_then(|item| item.downcast::<glib::BoxedAnyObject>().ok())
                .expect("GTK list row must be backed by BoxedAnyObject");
            let next_item = item_object.borrow::<I>().clone();

            let state_ptr = unsafe { list_item.data::<RowState<I>>(ROW_STATE_KEY) };
            let state = match state_ptr {
                Some(state) => unsafe { state.as_ref() },
                None => {
                    let state =
                        create_row_state(scope.clone(), parent_id, next_item.clone(), &factory);
                    unsafe {
                        list_item.set_data(ROW_STATE_KEY, state);
                    }
                    unsafe {
                        list_item
                            .data::<RowState<I>>(ROW_STATE_KEY)
                            .expect("row state must exist after setup")
                            .as_ref()
                    }
                }
            };

            state.item_signal.update_with(|current| {
                if comparator.are_content_the_same(current, &next_item) {
                    false
                } else {
                    *current = next_item.clone();
                    true
                }
            });

            let needs_attach = list_item
                .child()
                .map(|child| child != state.widget)
                .unwrap_or(true);
            if needs_attach {
                list_item.set_child(Some(&state.widget));
            }
        }
    });

    item_factory.connect_unbind(|_, list_item| {
        let list_item = as_list_item(list_item, "unbind");
        list_item.set_child(Option::<&gtk4::Widget>::None);
    });

    item_factory.connect_teardown({
        let scope = scope.clone();

        move |_, list_item| {
            let list_item = as_list_item(list_item, "teardown");
            list_item.set_child(Option::<&gtk4::Widget>::None);

            if let Some(state) = unsafe { list_item.steal_data::<RowState<I>>(ROW_STATE_KEY) } {
                scope.dispose_component(state.component_id);
            }
        }
    });

    let store_for_effect = store.clone();
    ctx.create_effect(move |_, previous_items: Option<Vec<I>>| {
        let data = list_data.read();
        let next_items = (0..data.count())
            .filter_map(|index| data.get_item(index).cloned())
            .collect::<Vec<_>>();

        if let Some(previous_items) = previous_items.as_ref() {
            sync_store(
                &store_for_effect,
                previous_items,
                &next_items,
                comparator.as_ref(),
            );
        } else {
            replace_all_items(&store_for_effect, &next_items);
        }

        next_items
    });

    let selection = NoSelection::new(Some(store.clone()));
    let list_view = GtkListView::builder()
        .model(&selection)
        .factory(&item_factory)
        .orientation(match orientation {
            ListOrientation::Vertical => GtkOrientation::Vertical,
            ListOrientation::Horizontal => GtkOrientation::Horizontal,
        })
        .single_click_activate(false)
        .build();
    list_view.set_vexpand(true);
    list_view.set_hexpand(true);

    let scroll = ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_hexpand(true);
    scroll.set_child(Some(&list_view));

    let scroll_widget: gtk4::Widget = scroll.clone().upcast();
    if let Some(child_entry_signal) = ctx.use_context(&CHILD_WIDGET) {
        let box_modifiers = ctx.use_context(&BOX_MODIFIERS);
        let flex_parent_data = ctx.use_context(&FLEX_PARENT_DATA);
        let scroll_widget = scroll_widget.clone();

        ctx.create_effect(move |_, _| {
            let layout = ChildLayoutInfo {
                box_modifiers: box_modifiers.read().unwrap_or_default(),
                flex: flex_parent_data.read().unwrap_or_default(),
            };
            child_entry_signal
                .read()
                .update_if_changes(Some(ChildWidgetEntry {
                    native: scroll_widget.clone(),
                    layout,
                }));
        });
    }
}

fn create_row_state<I>(
    scope: reactive_core::ReactiveScope,
    parent_id: reactive_core::ComponentId,
    initial_item: I,
    factory: &Rc<RefCell<dyn FnMut(ReadStoredSignal<I>) -> BoxedComponent>>,
) -> RowState<I>
where
    I: Clone + 'static,
{
    let item_signal = scope.create_signal(initial_item);
    let child_slot = scope.create_signal(None::<ChildWidgetEntry>);
    let user_component = factory.borrow_mut()(item_signal.clone().read_only());

    let wrapped = <Flex as Column>::new()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(move |ctx: &mut SetupContext| {
            user_component.setup(ctx);
        });

    let (component_id, ()) = scope.setup_child(parent_id, |child_ctx| {
        child_ctx.set_context(&CHILD_WIDGET, child_slot.clone().into_signal());
        Box::new(wrapped).setup(child_ctx);
    });

    let widget = child_slot
        .read()
        .expect("GTK list row component must register a child widget")
        .native;
    widget.set_vexpand(false);
    widget.set_hexpand(true);
    widget.set_valign(gtk4::Align::Start);

    RowState {
        item_signal,
        component_id,
        widget,
    }
}

fn as_list_item<'a>(list_item: &'a glib::Object, signal_name: &str) -> &'a gtk4::ListItem {
    list_item
        .downcast_ref::<gtk4::ListItem>()
        .unwrap_or_else(|| {
            panic!("signal-list-item-factory {signal_name} must receive gtk4::ListItem")
        })
}

fn replace_all_items<I>(store: &gio::ListStore, items: &[I])
where
    I: Clone + 'static,
{
    let additions = items
        .iter()
        .cloned()
        .map(glib::BoxedAnyObject::new)
        .collect::<Vec<_>>();
    store.splice(0, store.n_items(), &additions);
}

fn sync_store<I, Comp>(
    store: &gio::ListStore,
    previous_items: &[I],
    next_items: &[I],
    comparator: &Comp,
) where
    I: Clone + 'static,
    Comp: ListComparator<I>,
{
    let diff_result = diff(&previous_items, &next_items, comparator);
    if diff_result.ops.is_empty() {
        return;
    }

    if diff_result
        .ops
        .iter()
        .any(|op| matches!(op, DiffOp::Move { .. }))
    {
        replace_all_items(store, next_items);
        return;
    }

    let mut removes = Vec::new();
    let mut inserts = Vec::new();
    let mut changes = Vec::new();

    for op in diff_result.ops {
        match op {
            DiffOp::Remove { index, count } => removes.push((index, count)),
            DiffOp::Insert { index, count } => inserts.push((index, count)),
            DiffOp::Change { new_index, .. } => changes.push(new_index),
            DiffOp::Move { .. } => unreachable!("moves are handled by the full replace fallback"),
        }
    }

    removes.sort_unstable_by(|a, b| b.0.cmp(&a.0));
    for (index, count) in removes {
        store.splice(index as u32, count as u32, &[] as &[glib::Object]);
    }

    inserts.sort_unstable_by_key(|(index, _)| *index);
    for (index, count) in inserts {
        let additions = next_items[index..index + count]
            .iter()
            .cloned()
            .map(glib::BoxedAnyObject::new)
            .collect::<Vec<_>>();
        store.splice(index as u32, 0, &additions);
    }

    for index in changes {
        let updated = [glib::BoxedAnyObject::new(next_items[index].clone())];
        store.splice(index as u32, 1, &updated);
    }
}
