use crate::context::{CHILD_WIDGET, ChildWidgetEntry};
use crate::ui::flex::Flex;
use gtk4::prelude::*;
use reactive_core::{
    BoxedComponent, Component, ComponentId, IntoSignal, ReactiveScope, ReadStoredSignal,
    SetupContext, Signal, StoredSignal,
};
use std::cell::RefCell;
use std::rc::Rc;
use ui_core::layout::{BOX_MODIFIERS, ChildLayoutInfo, CrossAxisAlignment, FLEX_PARENT_DATA};
use ui_core::widgets::{
    Column, DiffOp, DiffResult, List, ListComparator, ListData, ListOrientation, diff,
};

// ---------------------------------------------------------------------------
// Cell — one live item in the list
// ---------------------------------------------------------------------------

struct Cell<I> {
    /// The item value at the time this cell was last updated.
    item: I,
    signal: StoredSignal<I>,
    component_id: ComponentId,
    widget: gtk4::Widget,
}

// ---------------------------------------------------------------------------
// Public struct
// ---------------------------------------------------------------------------

pub struct ListView {
    orientation: ListOrientation,
    setup_fn: Box<dyn FnOnce(ListOrientation, &mut SetupContext)>,
}

impl Component for ListView {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        (self.setup_fn)(self.orientation, ctx);
    }
}

// ---------------------------------------------------------------------------
// List trait impl
// ---------------------------------------------------------------------------

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
            Rc::new(RefCell::new(move |s| -> BoxedComponent {
                Box::new(component_factory(s))
            }));
        let comparator = Rc::new(list_comparator);

        ListView {
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

// ---------------------------------------------------------------------------
// Setup
// ---------------------------------------------------------------------------

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
    let gtk_orientation = match orientation {
        ListOrientation::Vertical => gtk4::Orientation::Vertical,
        ListOrientation::Horizontal => gtk4::Orientation::Horizontal,
    };

    let list_box = gtk4::Box::new(gtk_orientation, 0);
    let scroll = gtk4::ScrolledWindow::new();
    scroll.set_child(Some(&list_box));

    // Register the scroll window with the parent layout slot, inheriting any
    // layout modifiers (Padding, Expanded, etc.) placed above us.
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

    let parent_id = ctx.component_id();

    ctx.create_effect(move |scope, state: Option<Vec<Cell<I>>>| {
        let old_cells = state.unwrap_or_default();

        let data = list_data.read();
        let new_items: Vec<I> = (0..data.count())
            .filter_map(|i| data.get_item(i).cloned())
            .collect();

        let old_items: Vec<I> = old_cells.iter().map(|c| c.item.clone()).collect();
        let diff_result = diff(&old_items, &new_items, &*comparator);

        if diff_result.ops.is_empty() {
            return old_cells;
        }

        let new_cells = apply_diff(
            scope,
            parent_id,
            old_cells,
            &new_items,
            diff_result,
            &factory,
            &list_box,
        );

        // Sync the gtk::Box child order to match new_cells.
        let mut prev: Option<gtk4::Widget> = None;
        for cell in &new_cells {
            list_box.reorder_child_after(&cell.widget, prev.as_ref());
            prev = Some(cell.widget.clone());
        }

        new_cells
    });
}

// ---------------------------------------------------------------------------
// Diff application
// ---------------------------------------------------------------------------

fn apply_diff<I: Clone + 'static>(
    scope: &ReactiveScope,
    parent_id: ComponentId,
    old_cells: Vec<Cell<I>>,
    new_items: &[I],
    diff_result: DiffResult,
    factory: &Rc<RefCell<dyn FnMut(ReadStoredSignal<I>) -> BoxedComponent>>,
    list_box: &gtk4::Box,
) -> Vec<Cell<I>> {
    let n = old_cells.len();
    let m = new_items.len();

    // Parse ops into lookup tables.
    let mut removed = vec![false; n];
    let mut inserted = vec![false; m];
    // Maps new_index → old_index for Move and Change ops.
    let mut new_to_old: Vec<Option<usize>> = vec![None; m];
    // Which old cells need their signal updated (content changed).
    let mut old_needs_update = vec![false; n];

    for op in &diff_result.ops {
        match op {
            DiffOp::Remove { index, count } => {
                for i in *index..(*index + count) {
                    removed[i] = true;
                }
            }
            DiffOp::Insert { index, count } => {
                for i in *index..(*index + count) {
                    inserted[i] = true;
                }
            }
            DiffOp::Move {
                old_index,
                new_index,
                changed,
            } => {
                new_to_old[*new_index] = Some(*old_index);
                if *changed {
                    old_needs_update[*old_index] = true;
                }
            }
            DiffOp::Change {
                old_index,
                new_index,
            } => {
                new_to_old[*new_index] = Some(*old_index);
                old_needs_update[*old_index] = true;
            }
        }
    }

    // Assign implicit keeps: old indices not removed and not explicitly mapped
    // correspond in order to new indices not inserted and not explicitly mapped.
    let mut old_is_mapped = vec![false; n];
    for opt in &new_to_old {
        if let Some(old_i) = opt {
            old_is_mapped[*old_i] = true;
        }
    }
    let keep_olds: Vec<usize> = (0..n)
        .filter(|&i| !removed[i] && !old_is_mapped[i])
        .collect();
    let keep_news: Vec<usize> = (0..m)
        .filter(|&j| !inserted[j] && new_to_old[j].is_none())
        .collect();
    for (&old_i, &new_j) in keep_olds.iter().zip(keep_news.iter()) {
        new_to_old[new_j] = Some(old_i);
    }

    // Convert old_cells to Options so individual cells can be moved out.
    let mut old_cells_opt: Vec<Option<Cell<I>>> = old_cells.into_iter().map(Some).collect();

    // Dispose cells that are truly removed (not moved to a new position).
    for i in 0..n {
        if removed[i] {
            if let Some(cell) = old_cells_opt[i].take() {
                scope.dispose_component(cell.component_id);
                list_box.remove(&cell.widget);
            }
        }
    }

    // Build the new cell list.
    let mut new_cells: Vec<Cell<I>> = Vec::with_capacity(m);

    for new_j in 0..m {
        if let Some(old_i) = new_to_old[new_j] {
            // Reuse an existing cell (keep, change, or move).
            let cell = old_cells_opt[old_i]
                .take()
                .expect("each mapped old cell used exactly once");
            if old_needs_update[old_i] {
                cell.signal.update(new_items[new_j].clone());
            }
            new_cells.push(Cell {
                item: new_items[new_j].clone(),
                signal: cell.signal,
                component_id: cell.component_id,
                widget: cell.widget,
            });
        } else {
            // Fresh insert: create signal, set up component, extract widget.
            let new_item = new_items[new_j].clone();
            let signal = scope.create_signal(new_item.clone());
            let slot: StoredSignal<Option<ChildWidgetEntry>> = scope.create_signal(None);

            let (cid, widget) = scope.setup_child(parent_id, |child_ctx| {
                child_ctx.set_context(&CHILD_WIDGET, slot.clone().into_signal());

                // Always wrap in a vertical Column so that ConstraintHost is
                // guaranteed to be in the hierarchy.  This ensures Padding,
                // Expanded, and other layout modifiers work correctly even when
                // the factory doesn't supply a Row/Column itself.
                let user_component = factory.borrow_mut()(signal.clone().read_only());
                let wrapped = <Flex as Column>::new()
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .child(move |ctx: &mut SetupContext| {
                        user_component.setup(ctx);
                    });
                Box::new(wrapped).setup(child_ctx);

                slot.read()
                    .map(|e| e.native.clone())
                    .expect("factory component must produce a widget")
            });

            list_box.append(&widget);
            // Prevent vexpand from propagating up through the ConstraintHost
            // hierarchy (wrapping Labels with set_wrap(true) can set vexpand=true,
            // which would cause gtk4::Box to distribute the full viewport height
            // equally among cells instead of using each cell's natural height).
            widget.set_vexpand(false);
            widget.set_hexpand(true);

            new_cells.push(Cell {
                item: new_item,
                signal,
                component_id: cid,
                widget,
            });
        }
    }

    new_cells
}
