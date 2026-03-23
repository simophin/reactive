mod components;
mod context;
mod effects;
mod resources;
mod tick;
pub(crate) mod trackers;

use crate::component::SetupContext;

pub use resources::ResourceState;

use crate::component_scope::{ComponentId, ComponentScope};
use crate::signal::{SignalId, StoredSignal, remove_signal};
use slotmap::SlotMap;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

pub(crate) use trackers::{ActiveSignalTracker, DirtySignalSet};

pub(crate) struct ReactiveScopeData {
    pub(crate) components: SlotMap<ComponentId, ComponentScope>,
    pub(crate) root: Vec<ComponentId>,
    pub(crate) dirty_signals: DirtySignalSet,
    pub(crate) active_signal_tracker: ActiveSignalTracker,
    pub(crate) owned_signals: Vec<SignalId>,
}

impl Default for ReactiveScopeData {
    fn default() -> Self {
        Self {
            components: SlotMap::with_key(),
            root: Vec::new(),
            dirty_signals: DirtySignalSet::default(),
            active_signal_tracker: ActiveSignalTracker::default(),
            owned_signals: Vec::new(),
        }
    }
}

impl Drop for ReactiveScopeData {
    fn drop(&mut self) {
        for id in &self.owned_signals {
            remove_signal(*id);
        }
    }
}

/// A handle to the reactive runtime. Cheap to clone — all clones share the
/// same underlying state via `Rc<RefCell<...>>`.
#[derive(Clone)]
pub struct ReactiveScope(pub(crate) Rc<RefCell<ReactiveScopeData>>);

/// A weak reference to a [`ReactiveScope`] that does not prevent cleanup.
pub struct WeakReactiveScope(Weak<RefCell<ReactiveScopeData>>);

impl Default for ReactiveScope {
    fn default() -> Self {
        ReactiveScope(Rc::new(RefCell::new(ReactiveScopeData::default())))
    }
}

impl ReactiveScope {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn downgrade(&self) -> WeakReactiveScope {
        WeakReactiveScope(Rc::downgrade(&self.0))
    }

    /// Create a child component under `parent`, run `f` to set it up, and
    /// return both the new child's [`ComponentId`] and `f`'s return value.
    ///
    /// This is the hook that lets code running inside an effect (which
    /// receives `&ReactiveScope`) create new component scopes — the same
    /// capability that `SetupContext::child` provides during normal setup.
    pub fn setup_child<R>(
        &self,
        parent: ComponentId,
        f: impl FnOnce(&mut SetupContext) -> R,
    ) -> (ComponentId, R) {
        let child_id = self.create_child_component(Some(parent));
        let mut ctx = SetupContext {
            scope: self.clone(),
            component_id: child_id,
        };
        let r = f(&mut ctx);
        (child_id, r)
    }

    pub fn create_signal<T: 'static>(&self, initial: T) -> StoredSignal<T> {
        let signal = StoredSignal::new(initial, self.downgrade());
        self.0.borrow_mut().owned_signals.push(signal.id());
        signal
    }
}

impl WeakReactiveScope {
    pub fn upgrade(&self) -> Option<ReactiveScope> {
        self.0.upgrade().map(ReactiveScope)
    }
}
