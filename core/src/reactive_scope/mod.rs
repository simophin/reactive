mod components;
mod context;
mod effects;
mod resources;
mod tick;
pub(crate) mod trackers;

pub use resources::ResourceState;

use crate::component_scope::{ComponentId, ComponentScope};
use crate::signal::{SignalId, StoredSignal, remove_signal};
use slotmap::SlotMap;

pub(crate) use trackers::{
    ActiveSignalTracker, DirtySignalSet, WeakActiveSignalTracker, WeakDirtySignalSet,
};

#[derive(Default)]
pub struct ReactiveScope {
    components: SlotMap<ComponentId, ComponentScope>,
    root: Vec<ComponentId>,
    dirty_signals: DirtySignalSet,
    active_signal_tracker: ActiveSignalTracker,
    owned_signals: Vec<SignalId>,
}

impl Drop for ReactiveScope {
    fn drop(&mut self) {
        for id in &self.owned_signals {
            remove_signal(*id);
        }
    }
}

impl ReactiveScope {
    pub fn create_signal<T: 'static>(&mut self, initial: T) -> StoredSignal<T> {
        let signal = StoredSignal::new(
            initial,
            self.dirty_signals.downgrade(),
            self.active_signal_tracker.downgrade(),
        );
        self.owned_signals.push(signal.id());
        signal
    }
}
