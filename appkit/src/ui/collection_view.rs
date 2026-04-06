use std::cell::Cell;
use std::ffi::c_void;

use crate::view_component::AppKitViewBuilder;
use objc2::rc::Retained;
use objc2::{DefinedClass, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSCollectionView, NSCollectionViewDataSource, NSCollectionViewItem,
    NSIndexPathNSCollectionViewAdditions,
};
use objc2_foundation::{MainThreadMarker, NSIndexPath, NSInteger, NSObject, NSObjectProtocol};
use reactive_core::{Component, ReadStoredSignal, SetupContext, Signal};
use ui_core::widgets::{ListData, ListOrientation};

pub struct CollectionView {
    component: AppKitViewBuilder<NSCollectionView, ()>,
    orientation: ListOrientation,
}

impl Component for CollectionView {
    fn setup(self: Box<Self>, _ctx: &mut SetupContext) {
        todo!()
    }
}

impl ui_core::widgets::List for CollectionView {
    fn new<L, I, C>(
        _list_data: impl Signal<Value = L> + 'static,
        _component_factory: impl FnMut(ReadStoredSignal<I>) -> C + 'static,
    ) -> Self
    where
        L: ListData<I> + 'static,
        C: Component + 'static,
        I: 'static,
    {
        todo!()
    }

    fn orientation(self, _orientation: ListOrientation) -> Self {
        todo!()
    }
}

// // ---------------------------------------------------------------------------
// // Public API
// // ---------------------------------------------------------------------------
//
// /// A reactive, recycling wrapper around `NSCollectionView`.
// ///
// /// Each live `NSCollectionViewItem` carries its own companion `StoredSignal<Item>`
// /// stored as an ObjC ivar.  When AppKit recycles a cell we simply write new
// /// data through that signal; reactivity propagates the update without any view
// /// teardown.
// pub struct CollectionView<ItemsSignal, CellBuilder> {
//     items: ItemsSignal,
//     cell_builder: Rc<RefCell<CellBuilder>>,
//     layout: Retained<NSCollectionViewLayout>,
// }

// impl<ItemsSignal, CellBuilder> CollectionView<ItemsSignal, CellBuilder> {
//     pub fn new(
//         items: ItemsSignal,
//         cell_builder: CellBuilder,
//         layout: Retained<NSCollectionViewLayout>,
//     ) -> Self {
//         Self {
//             items,
//             cell_builder: Rc::new(RefCell::new(cell_builder)),
//             layout,
//         }
//     }
// }

// ---------------------------------------------------------------------------
// ReactiveCollectionViewItem — NSCollectionViewItem subclass with signal ivar
//
// `signal_ptr` stores a heap pointer to a `Box<StoredSignal<T>>`.
//
//   null  → cell is freshly allocated; no component set up yet.
//   non-null → component is live; dereference to get the signal.
//
// The Box is freed via an `on_cleanup` registered on the cell's own child
// component, so it is automatically reclaimed when the collection view
// component tree is torn down.
// ---------------------------------------------------------------------------

struct ReactiveItemIvars {
    signal_ptr: Cell<*const c_void>,
}

impl Default for ReactiveItemIvars {
    fn default() -> Self {
        // Null pointer = "not yet configured".  ObjC zero-initialises ivars on
        // alloc so this matches what AppKit sees when it creates cells itself.
        Self {
            signal_ptr: Cell::new(std::ptr::null()),
        }
    }
}

define_class!(
    #[unsafe(super(NSCollectionViewItem))]
    #[thread_kind = MainThreadOnly]
    #[name = "ReactiveCollectionViewItem"]
    #[ivars = ReactiveItemIvars]
    struct ReactiveItem;

    unsafe impl NSObjectProtocol for ReactiveItem {}
);

// ---------------------------------------------------------------------------
// ObjC helpers
// ---------------------------------------------------------------------------

fn make_index_path(item: usize) -> Retained<NSIndexPath> {
    NSIndexPath::indexPathForItem_inSection(item as NSInteger, 0)
}

// ---------------------------------------------------------------------------
// Type-erased data-source callbacks
// ---------------------------------------------------------------------------

struct DataSourceCallbacks {
    item_count: Box<dyn Fn() -> usize>,
    item_at: Box<dyn Fn(usize) -> Retained<NSCollectionViewItem>>,
}

// ---------------------------------------------------------------------------
// ObjC data-source class
// ---------------------------------------------------------------------------

struct DataSourceIvars {
    ptr: Cell<*const c_void>,
}

impl Default for DataSourceIvars {
    fn default() -> Self {
        Self {
            ptr: Cell::new(std::ptr::null()),
        }
    }
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "ReactiveCollectionViewDataSource"]
    #[ivars = DataSourceIvars]
    struct CollectionDataSource;

    unsafe impl NSObjectProtocol for CollectionDataSource {}

    unsafe impl NSCollectionViewDataSource for CollectionDataSource {
        #[unsafe(method(collectionView:numberOfItemsInSection:))]
        fn collection_view_number_of_items(
            &self,
            _cv: &NSCollectionView,
            _section: NSInteger,
        ) -> NSInteger {
            let ptr = self.ivars().ptr.get();
            if ptr.is_null() {
                return 0;
            }
            let cb = unsafe { &*(ptr as *const DataSourceCallbacks) };
            (cb.item_count)() as NSInteger
        }

        #[unsafe(method_id(collectionView:itemForRepresentedObjectAtIndexPath:))]
        fn collection_view_item_for_index_path(
            &self,
            _cv: &NSCollectionView,
            index_path: &NSIndexPath,
        ) -> Retained<NSCollectionViewItem> {
            let ptr = self.ivars().ptr.get();
            debug_assert!(!ptr.is_null());
            let cb = unsafe { &*(ptr as *const DataSourceCallbacks) };
            (cb.item_at)(index_path.item() as usize)
        }
    }
);

impl CollectionDataSource {
    fn new(mtm: MainThreadMarker, callbacks_ptr: *const DataSourceCallbacks) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(DataSourceIvars {
            ptr: Cell::new(callbacks_ptr as *const c_void),
        });
        unsafe { msg_send![super(this), init] }
    }
}

// ---------------------------------------------------------------------------
// Component impl
// ---------------------------------------------------------------------------

// impl<Item, ItemsSignal, CellBuilder, CellComponent> Component
//     for CollectionView<ItemsSignal, CellBuilder>
// where
//     Item: PartialEq + Clone + 'static,
//     ItemsSignal: Signal<Value = Rc<[Item]>> + 'static,
//     CellBuilder: FnMut(ReadStoredSignal<Item>) -> CellComponent + 'static,
//     CellComponent: Component + 'static,
// {
//     fn setup(self: Box<Self>, ctx: &mut SetupContext) {
//         let mtm =
//             MainThreadMarker::new().expect("CollectionView must be set up on the main thread");
//
//         let CollectionView {
//             items,
//             cell_builder,
//             layout,
//         } = *self;
//
//         // ── View hierarchy ─────────────────────────────────────────────────
//         let scroll_view = NSScrollView::new(mtm);
//         let collection_view = NSCollectionView::new(mtm);
//         collection_view.setCollectionViewLayout(Some(&*layout));
//         scroll_view.setDocumentView(Some(collection_view.as_ref()));
//
//         let sv_nsview: Retained<NSView> = scroll_view.clone().into_super();
//         if let Some(parent_signal) = ctx.use_context(&PARENT_VIEW) {
//             let parent = parent_signal.read();
//             parent.add_child(sv_nsview.clone());
//             ctx.on_cleanup({
//                 let sv = sv_nsview.clone();
//                 move || parent.remove_child(&sv)
//             });
//         }
//
//         // ── Register our subclass for AppKit's reuse queue ─────────────────
//         //
//         // Using `ReactiveItem` instead of the base `NSCollectionViewItem`
//         // ensures every cell vended by `makeItemWithIdentifier:forIndexPath:`
//         // carries our `signal_ptr` ivar.
//         let cell_identifier = NSString::from_str("cell");
//         unsafe {
//             let _: () = msg_send![
//                 &*collection_view,
//                 registerClass: ReactiveItem::class(),
//                 forItemWithIdentifier: &*cell_identifier
//             ];
//         }
//
//         // ── Current items snapshot ─────────────────────────────────────────
//         //
//         // Written by the reconciliation effect before `reloadData`; read by
//         // `item_at` to know which datum to bind to each cell.
//         let current_items: Rc<RefCell<Rc<[Item]>>> = Rc::new(RefCell::new(Rc::from([])));
//
//         // ── Clone scope for use inside the data-source callback ────────────
//         //
//         // `ReactiveScope` is `Rc<RefCell<...>>` — cheap to clone, all clones
//         // share the same runtime.  This lets `item_at` call `setup_child` and
//         // `create_signal` even though it runs outside any reactive effect.
//         let scope_for_item_at = ctx.scope();
//         let parent_id = ctx.component_id();
//
//         // ── Data-source callbacks ──────────────────────────────────────────
//         let callbacks = Box::new(DataSourceCallbacks {
//             item_count: Box::new({
//                 let items = Rc::clone(&current_items);
//                 move || items.borrow().len()
//             }),
//
//             item_at: Box::new({
//                 let items = Rc::clone(&current_items);
//                 let builder = Rc::clone(&cell_builder);
//                 let scope = scope_for_item_at;
//                 let cv = collection_view.clone();
//                 let identifier = cell_identifier.clone();
//                 move |index| {
//                     let ip = make_index_path(index);
//
//                     // Ask AppKit for a cell.  Because we registered `ReactiveItem`,
//                     // every returned instance has our `signal_ptr` ivar.
//                     let cv_item: Retained<NSCollectionViewItem> = unsafe {
//                         msg_send![
//                             &*cv,
//                             makeItemWithIdentifier: &*identifier,
//                             forIndexPath: &*ip
//                         ]
//                     };
//
//                     // Cast to our concrete subtype to reach the ivar.
//                     // SAFETY: all cells for this identifier are `ReactiveItem`.
//                     let reactive = unsafe {
//                         &*(&*cv_item as *const NSCollectionViewItem as *const ReactiveItem)
//                     };
//                     let raw = reactive.ivars().signal_ptr.get();
//
//                     if raw.is_null() {
//                         // ── Create path ─────────────────────────────────────
//                         // First time AppKit allocates this cell object.
//                         // Create a signal, store a stable heap pointer to it in
//                         // the ivar, then wire up the child component.
//
//                         let signal = scope.create_signal(items.borrow()[index].clone());
//
//                         // Heap-allocate a clone so the ivar holds a stable address.
//                         // The original `signal` is moved into `ReadStoredSignal` below.
//                         // The Box is freed via `on_cleanup` on the cell's child component.
//                         let raw = Box::into_raw(Box::new(signal.clone())) as *const c_void;
//                         reactive.ivars().signal_ptr.set(raw);
//
//                         let container = NSView::new(mtm);
//                         cv_item.setView(&*container);
//
//                         scope.setup_child(parent_id, |child_ctx| {
//                             child_ctx
//                                 .provide_context(&PARENT_VIEW, ViewParent::View(container.clone()));
//                             Box::new((builder.borrow_mut())(signal.read_only())).setup(child_ctx);
//
//                             // Free the signal Box when the collection view's
//                             // component tree is eventually disposed.
//                             child_ctx.on_cleanup(move || unsafe {
//                                 drop(Box::<StoredSignal<Item>>::from_raw(
//                                     raw as *mut StoredSignal<Item>,
//                                 ));
//                             });
//                         });
//                     } else {
//                         // ── Reuse path ──────────────────────────────────────
//                         // AppKit handed back a recycled cell.  The ivar already
//                         // points to the companion signal; push the new data
//                         // through it and let reactive effects do the rest.
//                         let signal = unsafe { &*(raw as *const StoredSignal<Item>) }.clone();
//                         signal.update_with(|curr| {
//                             let items = items.borrow();
//                             if &items[index] != curr {
//                                 *curr = items[index].clone();
//                                 true
//                             } else {
//                                 false
//                             }
//                         })
//                     }
//
//                     cv_item
//                 }
//             }),
//         });
//
//         let callbacks_ptr: *const DataSourceCallbacks = &*callbacks;
//         let datasource = CollectionDataSource::new(mtm, callbacks_ptr);
//         collection_view.setDataSource(Some(ProtocolObject::from_ref(&*datasource)));
//
//         // ── Reconciliation effect ──────────────────────────────────────────
//         //
//         // On every items change: snapshot into `current_items`, then reload.
//         // AppKit drives `item_at` for each visible cell; create/reuse is
//         // handled there transparently.
//         let cv = collection_view.clone();
//         ctx.create_effect(move |_: &ReactiveScope, _: Option<()>| {
//             *current_items.borrow_mut() = items.read();
//             cv.reloadData();
//         });
//
//         // Cleanup: drop datasource first so no ObjC callbacks fire after the
//         // `callbacks` Box is freed.
//         ctx.on_cleanup(move || {
//             drop(datasource);
//             drop(callbacks);
//         });
//     }
// }
