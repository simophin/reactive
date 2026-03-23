use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::ffi::c_void;
use std::hash::Hash;
use std::rc::Rc;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{DefinedClass, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSCollectionView, NSCollectionViewDataSource, NSCollectionViewItem, NSCollectionViewLayout,
    NSIndexPathNSCollectionViewAdditions, NSScrollView, NSView,
};
use objc2_foundation::{
    MainThreadMarker, NSIndexPath, NSInteger, NSMutableSet, NSObject, NSObjectProtocol, NSSet,
};
use reactive_core::{Component, ComponentId, ReadSignal, SetupContext, Signal, StoredSignal};

use super::context::{PARENT_VIEW, ViewParent};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// A reactive, signal-driven wrapper around `NSCollectionView`.
///
/// Items are reconciled by key, so components are never torn down and
/// recreated for an item that merely moved or changed its data.
///
/// | Change kind             | Who handles it              |
/// |-------------------------|-----------------------------|
/// | Item data changed       | Reactive signal (no reload) |
/// | Item added              | `insertItemsAtIndexPaths`   |
/// | Item removed            | `deleteItemsAtIndexPaths`   |
/// | Item moved              | `moveItemAtIndexPath:to:`   |
pub struct CollectionView<ItemsSignal, KeyFn, CellBuilder> {
    items: ItemsSignal,
    key_fn: KeyFn,
    cell_builder: Rc<RefCell<CellBuilder>>,
    layout: Retained<NSCollectionViewLayout>,
}

impl<ItemsSignal, KeyFn, CellBuilder> CollectionView<ItemsSignal, KeyFn, CellBuilder> {
    pub fn new(
        items: ItemsSignal,
        key_fn: KeyFn,
        cell_builder: CellBuilder,
        layout: Retained<NSCollectionViewLayout>,
    ) -> Self {
        Self {
            items,
            key_fn,
            cell_builder: Rc::new(RefCell::new(cell_builder)),
            layout,
        }
    }
}

// ---------------------------------------------------------------------------
// Pool slot — one per live item, keyed by item identity
// ---------------------------------------------------------------------------

struct CellSlot<T: Clone + 'static> {
    signal: StoredSignal<T>,
    item: Retained<NSCollectionViewItem>,
    component_id: ComponentId,
}

// ---------------------------------------------------------------------------
// Shared pool state
// ---------------------------------------------------------------------------

/// The live pool, shared between the reconciliation effect and the
/// ObjC data source.
struct PoolState<T: Clone + 'static, K: Eq + Hash + Clone + 'static> {
    /// Keyed map of live slots.
    slots: HashMap<K, CellSlot<T>>,
    /// Ordered list of keys, mirrors the current datasource order.
    key_order: Vec<K>,
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> Default for PoolState<T, K> {
    fn default() -> Self {
        Self {
            slots: HashMap::new(),
            key_order: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Diff engine
// ---------------------------------------------------------------------------

struct BatchUpdate<K> {
    /// Deleted items: (key, index in the PRE-update order).
    deleted: Vec<(K, usize)>,
    /// Inserted items: (key, index in the POST-update order).
    inserted: Vec<(K, usize)>,
    /// Survivors that moved: (index in PRE-update order, index in POST-update order).
    moved: Vec<(usize, usize)>,
}

fn diff<K: Eq + Hash + Clone>(old_order: &[K], new_order: &[K]) -> BatchUpdate<K> {
    let old_set: HashSet<&K> = old_order.iter().collect();
    let new_set: HashSet<&K> = new_order.iter().collect();

    let deleted = old_order
        .iter()
        .enumerate()
        .filter(|(_, k)| !new_set.contains(k))
        .map(|(i, k)| (k.clone(), i))
        .collect();

    let inserted = new_order
        .iter()
        .enumerate()
        .filter(|(_, k)| !old_set.contains(k))
        .map(|(i, k)| (k.clone(), i))
        .collect();

    // Build old-position lookup for survivors only.
    let old_pos: HashMap<&K, usize> = old_order
        .iter()
        .enumerate()
        .filter(|(_, k)| new_set.contains(k))
        .map(|(i, k)| (k, i))
        .collect();

    let moved = new_order
        .iter()
        .enumerate()
        .filter(|(_, k)| old_set.contains(k))
        .filter_map(|(new_i, k)| old_pos.get(k).map(|&old_i| (old_i, new_i)))
        .filter(|(old_i, new_i)| old_i != new_i)
        .collect();

    BatchUpdate {
        deleted,
        inserted,
        moved,
    }
}

// ---------------------------------------------------------------------------
// ObjC helpers: NSIndexPath / NSSet<NSIndexPath>
// ---------------------------------------------------------------------------

fn index_path(item: usize) -> Retained<NSIndexPath> {
    NSIndexPath::indexPathForItem_inSection(item as NSInteger, 0)
}

fn index_path_set(indices: impl IntoIterator<Item = usize>) -> Retained<NSSet<NSIndexPath>> {
    let set = NSMutableSet::new();
    for i in indices {
        set.addObject(&*index_path(i));
    }
    unsafe { Retained::cast_unchecked(set) }
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
    /// Raw pointer to a `Box<DataSourceCallbacks>` owned by the reactive
    /// scope via `on_cleanup`.  Null until `new` is called.
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

impl<Item, ItemsSignal, KeyFn, CellBuilder, Key, CellComponent> Component
    for CollectionView<ItemsSignal, KeyFn, CellBuilder>
where
    Item: Clone + 'static,
    ItemsSignal: Signal<Value = Rc<[Item]>> + 'static,
    KeyFn: FnMut(&Item) -> Key + 'static,
    CellBuilder: FnMut(ReadSignal<Item>) -> CellComponent + 'static,
    CellComponent: Component + 'static,
    Key: Eq + Hash + Clone + 'static,
{
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let mtm =
            MainThreadMarker::new().expect("CollectionView must be set up on the main thread");

        let CollectionView {
            items,
            mut key_fn,
            cell_builder,
            layout,
        } = *self;

        // ── View hierarchy ─────────────────────────────────────────────────
        let scroll_view = NSScrollView::new(mtm);
        let collection_view = NSCollectionView::new(mtm);
        collection_view.setCollectionViewLayout(Some(&*layout));
        scroll_view.setDocumentView(Some(collection_view.as_ref()));

        let sv_nsview: Retained<NSView> = scroll_view.clone().into_super();

        if let Some(parent_signal) = ctx.use_context(&PARENT_VIEW) {
            let parent = parent_signal.read();
            parent.add_child(sv_nsview.clone());
            ctx.on_cleanup({
                let sv = sv_nsview.clone();
                move || parent.remove_child(&sv)
            });
        }

        // ── Shared pool ────────────────────────────────────────────────────
        let pool: Rc<RefCell<PoolState<Item, Key>>> = Rc::default();

        // ── Type-erased callbacks for the ObjC data source ─────────────────
        let callbacks = Box::new(DataSourceCallbacks {
            item_count: Box::new({
                let p = Rc::clone(&pool);
                move || p.borrow().key_order.len()
            }),
            item_at: Box::new({
                let p = Rc::clone(&pool);
                move |index| {
                    let p = p.borrow();
                    p.slots[&p.key_order[index]].item.clone()
                }
            }),
        });
        let callbacks_ptr: *const DataSourceCallbacks = &*callbacks;

        let datasource = CollectionDataSource::new(mtm, callbacks_ptr);
        collection_view.setDataSource(Some(ProtocolObject::from_ref(&*datasource)));

        // ── Reconciliation effect ──────────────────────────────────────────
        let cv = collection_view.clone();
        let parent_id = ctx.component_id();

        ctx.create_effect(move |scope: &reactive_core::ReactiveScope, _: Option<()>| {
            let new_items = items.read();
            let new_keys: Vec<Key> = new_items.iter().map(|item| key_fn(item)).collect();

            // Compute diff against current order before touching the pool.
            let update = diff(&pool.borrow().key_order, &new_keys);
            let is_first_render = pool.borrow().key_order.is_empty() && !new_keys.is_empty();

            // --- mutate pool (must finish before NSCollectionView calls) ---
            {
                let mut pool = pool.borrow_mut();

                // Dispose slots for deleted items.
                for (key, _) in &update.deleted {
                    if let Some(slot) = pool.slots.remove(key) {
                        scope.dispose_component(slot.component_id);
                    }
                }

                // Create new slots for inserted items.
                for (key, new_i) in &update.inserted {
                    let item_data = &new_items[*new_i];
                    let signal = scope.create_signal(item_data.clone());
                    let read_signal = ReadSignal::from(signal);

                    let container = NSView::new(mtm);
                    let cv_item = NSCollectionViewItem::new(mtm);
                    cv_item.setView(&*container);

                    let builder = Rc::clone(&cell_builder);
                    let (child_id, ()) = scope.setup_child(parent_id, |child_ctx| {
                        child_ctx
                            .provide_context(&PARENT_VIEW, ViewParent::Window(container.clone()));
                        Box::new((builder.borrow_mut())(read_signal)).setup(child_ctx);
                    });

                    pool.slots.insert(
                        key.clone(),
                        CellSlot {
                            signal,
                            item: cv_item,
                            component_id: child_id,
                        },
                    );
                }

                // Commit the new key ordering — the data source reads this.
                pool.key_order = new_keys;

                // Rebind all surviving slots.  When nothing structural changed,
                // this is the only work done: no NSCollectionView call at all.
                for item_data in new_items.iter() {
                    let key = key_fn(item_data);
                    if let Some(slot) = pool.slots.get(&key) {
                        slot.signal.set_and_notify_changes(item_data.clone());
                    }
                }
            } // <-- borrow_mut released here

            // --- notify NSCollectionView of structural changes ------------
            let is_structural = !update.deleted.is_empty()
                || !update.inserted.is_empty()
                || !update.moved.is_empty();

            if is_first_render {
                // Simple full load for the initial render.
                cv.reloadData();
            } else if is_structural {
                // Animated incremental update.
                //
                // Index semantics (same as UICollectionView):
                //   deleted  → pre-update positions
                //   inserted → post-update positions
                //   moved    → (pre-update pos, post-update pos)
                let deleted_idx: Vec<usize> = update.deleted.iter().map(|(_, i)| *i).collect();
                let inserted_idx: Vec<usize> = update.inserted.iter().map(|(_, i)| *i).collect();
                let moved_pairs = update.moved;

                let cv2 = cv.clone();
                cv.performBatchUpdates_completionHandler(
                    Some(&RcBlock::new(move || {
                        if !deleted_idx.is_empty() {
                            cv2.deleteItemsAtIndexPaths(&index_path_set(
                                deleted_idx.iter().copied(),
                            ));
                        }
                        if !inserted_idx.is_empty() {
                            cv2.insertItemsAtIndexPaths(&index_path_set(
                                inserted_idx.iter().copied(),
                            ));
                        }
                        for (from, to) in &moved_pairs {
                            cv2.moveItemAtIndexPath_toIndexPath(
                                &index_path(*from),
                                &index_path(*to),
                            );
                        }
                    })),
                    None,
                );
            }
            // Pure data changes: reactive effects handle everything, no reload.
        });

        // Cleanup order is LIFO: datasource drops first (preventing any further
        // calls into the callbacks), then callbacks drops safely.
        ctx.on_cleanup(move || {
            drop(datasource);
            drop(callbacks);
        });
    }
}
