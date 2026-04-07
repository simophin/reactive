use std::cell::Cell;
use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;

use crate::context::{CHILD_VIEW, ChildViewEntry};
use crate::ui::flex::Flex;
use crate::ui::layout::{activate_constraints, pin_edges};
use crate::view_component::AppKitViewBuilder;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{DefinedClass, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSControlTextEditingDelegate, NSScrollView, NSTableColumn, NSTableView,
    NSTableViewColumnAutoresizingStyle, NSTableViewDataSource, NSTableViewDelegate,
    NSTableViewSelectionHighlightStyle, NSUserInterfaceItemIdentification, NSView,
};
use objc2_foundation::{MainThreadMarker, NSInteger, NSObject, NSObjectProtocol, NSSize, NSString};
use reactive_core::{
    BoxedComponent, Component, IntoSignal, ReadStoredSignal, SetupContext, Signal, StoredSignal,
};
use ui_core::layout::CrossAxisAlignment;
use ui_core::widgets::{Column, List, ListComparator, ListData, ListOrientation};

// ---------------------------------------------------------------------------
// ReactiveCellView — NSView subclass that stores a per-cell signal pointer
//
//   null     → cell is fresh; no component set up yet
//   non-null → component is live; dereference to get StoredSignal<I>
//
// The Box<StoredSignal<I>> is freed in an on_cleanup registered during
// component setup, so it is reclaimed when the list is torn down.
// ---------------------------------------------------------------------------

struct CellViewIvars {
    signal_ptr: Cell<*const c_void>,
}

impl Default for CellViewIvars {
    fn default() -> Self {
        Self {
            signal_ptr: Cell::new(std::ptr::null()),
        }
    }
}

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "ReactiveListCellView"]
    #[ivars = CellViewIvars]
    struct ReactiveCellView;

    unsafe impl NSObjectProtocol for ReactiveCellView {}
);

impl ReactiveCellView {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(CellViewIvars::default());
        unsafe { msg_send![super(this), init] }
    }
}

// ---------------------------------------------------------------------------
// Type-erased table callbacks
// ---------------------------------------------------------------------------

struct TableCallbacks {
    row_count: Box<dyn Fn() -> usize>,
    view_for_row: Box<dyn Fn(&NSTableView, usize) -> Retained<NSView>>,
}

// ---------------------------------------------------------------------------
// TableDataSource — NSObject subclass that wires NSTableView data + delegate
// ---------------------------------------------------------------------------

struct TableDataSourceIvars {
    ptr: Cell<*const c_void>,
}

impl Default for TableDataSourceIvars {
    fn default() -> Self {
        Self {
            ptr: Cell::new(std::ptr::null()),
        }
    }
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "ReactiveListTableDataSource"]
    #[ivars = TableDataSourceIvars]
    struct TableDataSource;

    unsafe impl NSObjectProtocol for TableDataSource {}

    unsafe impl NSTableViewDataSource for TableDataSource {
        #[allow(non_snake_case)]
        #[unsafe(method(numberOfRowsInTableView:))]
        fn numberOfRowsInTableView(&self, _tv: &NSTableView) -> NSInteger {
            let ptr = self.ivars().ptr.get();
            if ptr.is_null() {
                return 0;
            }
            let cb = unsafe { &*(ptr as *const TableCallbacks) };
            (cb.row_count)() as NSInteger
        }
    }

    unsafe impl NSControlTextEditingDelegate for TableDataSource {}

    unsafe impl NSTableViewDelegate for TableDataSource {
        #[allow(non_snake_case)]
        #[unsafe(method_id(tableView:viewForTableColumn:row:))]
        fn tableView_viewForTableColumn_row(
            &self,
            tv: &NSTableView,
            _column: Option<&NSTableColumn>,
            row: NSInteger,
        ) -> Option<Retained<NSView>> {
            let ptr = self.ivars().ptr.get();
            debug_assert!(!ptr.is_null());
            let cb = unsafe { &*(ptr as *const TableCallbacks) };
            Some((cb.view_for_row)(tv, row as usize))
        }
    }
);

impl TableDataSource {
    fn new(mtm: MainThreadMarker, callbacks_ptr: *const TableCallbacks) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(TableDataSourceIvars {
            ptr: Cell::new(callbacks_ptr as *const c_void),
        });
        unsafe { msg_send![super(this), init] }
    }
}

// ---------------------------------------------------------------------------
// Public struct
// ---------------------------------------------------------------------------

pub struct CollectionView {
    orientation: ListOrientation,
    setup_fn: Box<dyn FnOnce(ListOrientation, &mut SetupContext)>,
}

impl Component for CollectionView {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        (self.setup_fn)(self.orientation, ctx);
    }
}

// ---------------------------------------------------------------------------
// List trait impl
// ---------------------------------------------------------------------------

impl List for CollectionView {
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

        CollectionView {
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

fn scroll_view_into_nsview(sv: Retained<NSScrollView>) -> Retained<NSView> {
    sv.into_super()
}

fn setup_list<I, L, Comp>(
    ctx: &mut SetupContext,
    _orientation: ListOrientation,
    list_data: impl Signal<Value = L> + 'static,
    factory: Rc<RefCell<dyn FnMut(ReadStoredSignal<I>) -> BoxedComponent>>,
    comparator: Rc<Comp>,
) where
    I: Clone + 'static,
    L: ListData<I> + 'static,
    Comp: ListComparator<I> + 'static,
{
    let mtm = MainThreadMarker::new().expect("CollectionView must be set up on the main thread");

    let table_view = NSTableView::new(mtm);

    // Appearance: plain list, no header, no selection highlight, no grid lines.
    unsafe {
        let _: () = msg_send![&*table_view, setHeaderView: std::ptr::null::<AnyObject>()];
    }
    table_view.setSelectionHighlightStyle(NSTableViewSelectionHighlightStyle::None);
    table_view.setUsesAutomaticRowHeights(true);
    table_view.setIntercellSpacing(NSSize {
        width: 0.0,
        height: 0.0,
    });

    // Single column that fills all available width.
    let col_id = NSString::from_str("main");
    let column = NSTableColumn::initWithIdentifier(NSTableColumn::alloc(mtm), &col_id);
    column.setMinWidth(0.0);
    column.setMaxWidth(f64::MAX);
    table_view.addTableColumn(&column);
    table_view.setColumnAutoresizingStyle(
        NSTableViewColumnAutoresizingStyle::LastColumnOnlyAutoresizingStyle,
    );

    // Wrap in a scroll view and register it with the parent layout.
    let scroll_view = AppKitViewBuilder::create_no_child(
        move |_| {
            let mtm = MainThreadMarker::new().unwrap();
            NSScrollView::new(mtm)
        },
        scroll_view_into_nsview,
    )
    .debug_identifier("ListView")
    .setup(ctx);

    scroll_view.setDocumentView(Some(&**table_view));
    scroll_view.setHasVerticalScroller(true);

    // Snapshot of current items — written by the reconciliation effect and
    // read by the data-source callbacks (which run from AppKit, outside effects).
    let current_items: Rc<RefCell<Vec<I>>> = Rc::new(RefCell::new(Vec::new()));

    let scope = ctx.scope();
    let parent_id = ctx.component_id();
    let cell_id = NSString::from_str("cell");

    let callbacks = Box::new(TableCallbacks {
        row_count: {
            let items = Rc::clone(&current_items);
            Box::new(move || items.borrow().len())
        },

        view_for_row: {
            let items = Rc::clone(&current_items);
            let factory = Rc::clone(&factory);
            let comparator = Rc::clone(&comparator);
            let scope = scope.clone();
            let cell_id = cell_id.clone();

            Box::new(move |tv: &NSTableView, index: usize| {
                // AppKit always calls this from the main thread.
                let mtm = unsafe { MainThreadMarker::new_unchecked() };

                // Try to dequeue a recycled cell.
                let maybe_recycled = unsafe { tv.makeViewWithIdentifier_owner(&cell_id, None) };

                if let Some(recycled) = maybe_recycled {
                    // ── Reuse path ────────────────────────────────────────────
                    // Cast back to our subclass to reach the signal ivar.
                    let cell =
                        unsafe { &*(&*recycled as *const NSView as *const ReactiveCellView) };
                    let raw = cell.ivars().signal_ptr.get();
                    if !raw.is_null() {
                        let signal = unsafe { &*(raw as *const StoredSignal<I>) };
                        let new_item = &items.borrow()[index];
                        signal.update_with(|curr| {
                            if !comparator.are_content_the_same(curr, new_item) {
                                *curr = new_item.clone();
                                true
                            } else {
                                false
                            }
                        });
                    }
                    recycled
                } else {
                    // ── Create path ───────────────────────────────────────────
                    let item_val = items.borrow()[index].clone();
                    let signal = scope.create_signal(item_val);
                    let raw = Box::into_raw(Box::new(signal.clone())) as *const c_void;

                    let cell_view = ReactiveCellView::new(mtm);
                    cell_view.ivars().signal_ptr.set(raw);
                    // Stamp with the reuse identifier.
                    cell_view.setIdentifier(Some(&cell_id));

                    let slot: StoredSignal<Option<ChildViewEntry>> = scope.create_signal(None);
                    scope.setup_child(parent_id, |child_ctx| {
                        child_ctx.set_context(&CHILD_VIEW, slot.clone().into_signal());

                        // Always wrap in a vertical Column so that ReactiveLayoutView
                        // is guaranteed to be in the hierarchy. This ensures Padding,
                        // Expanded, and intrinsicContentSize work correctly even when
                        // the factory doesn't supply a Row/Column itself.
                        let user_component = factory.borrow_mut()(signal.clone().read_only());
                        let wrapped = <Flex as Column>::new()
                            .cross_axis_alignment(CrossAxisAlignment::Stretch)
                            .child(move |ctx: &mut SetupContext| {
                                user_component.setup(ctx);
                            });
                        Box::new(wrapped).setup(child_ctx);

                        // Free the heap-boxed signal when this component is disposed.
                        child_ctx.on_cleanup(move || unsafe {
                            drop(Box::<StoredSignal<I>>::from_raw(raw as *mut _));
                        });
                    });

                    // Wire the child view into the cell with Auto Layout constraints.
                    let cell_view_ns: Retained<NSView> = cell_view.into_super();
                    if let Some(entry) = slot.read() {
                        cell_view_ns.addSubview(&*entry.native);
                        activate_constraints(&pin_edges(&*entry.native, &cell_view_ns));
                    }

                    cell_view_ns
                }
            })
        },
    });

    let callbacks_ptr: *const TableCallbacks = &*callbacks;
    let datasource = TableDataSource::new(mtm, callbacks_ptr);
    unsafe {
        table_view.setDataSource(Some(ProtocolObject::from_ref(&*datasource)));
        table_view.setDelegate(Some(ProtocolObject::from_ref(&*datasource)));
    }

    // Reconciliation effect: on every items change, snapshot and tell the
    // table to reload. AppKit then drives view_for_row for each visible row.
    let tv = table_view.clone();
    ctx.create_effect(move |_, _: Option<()>| {
        let data = list_data.read();
        let new_items: Vec<I> = (0..data.count())
            .filter_map(|i| data.get_item(i).cloned())
            .collect();
        *current_items.borrow_mut() = new_items;
        tv.reloadData();
    });

    // Keep the datasource and callbacks alive for the component's lifetime.
    ctx.on_cleanup(move || {
        drop(datasource);
        drop(callbacks);
    });
}
