use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::reactive_scope::{ActiveSignalTracker, DirtySignalSet};
use crate::signal::{Signal, SignalId};

static SIGNAL_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

// ---------------------------------------------------------------------------
// StoredSignal
// ---------------------------------------------------------------------------

pub struct StoredSignal<T> {
    id: SignalId,
    value: Rc<RefCell<T>>,
    dirty_signal_set: DirtySignalSet,
    active_signal_tracker: ActiveSignalTracker,
}

impl<T> Clone for StoredSignal<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            value: Rc::clone(&self.value),
            dirty_signal_set: self.dirty_signal_set.clone(),
            active_signal_tracker: self.active_signal_tracker.clone(),
        }
    }
}

impl<T> StoredSignal<T> {
    pub(crate) fn new(
        init: T,
        dirty_signal_set: DirtySignalSet,
        active_signal_tracker: ActiveSignalTracker,
    ) -> Self {
        Self {
            id: SIGNAL_ID_COUNTER.fetch_add(1, Ordering::Relaxed),
            value: Rc::new(RefCell::new(init)),
            dirty_signal_set,
            active_signal_tracker,
        }
    }

    pub fn id(&self) -> SignalId {
        self.id
    }

    pub fn update(&self, f: impl FnOnce(&mut T) -> bool) {
        if f(&mut *self.value.borrow_mut()) {
            self.dirty_signal_set.mark_dirty(self.id);
        }
    }

    pub fn set_and_notify_changes(&self, value: T) {
        self.update(|v| {
            *v = value;
            true
        });
    }

    pub fn update_if_changes(&self, value: T)
    where
        T: PartialEq,
    {
        self.update(|v| {
            if v != &value {
                *v = value;
                true
            } else {
                false
            }
        });
    }
}

impl<T: Clone + 'static> Signal for StoredSignal<T> {
    type Value = T;

    fn read(&self) -> T {
        let v = (*self.value.borrow()).clone();
        self.active_signal_tracker.on_accessed(self.id);
        v
    }
}

// ---------------------------------------------------------------------------
// BoxedStoredSignal — type-erased handle used by the context system
// ---------------------------------------------------------------------------

trait RawStoreSignal: Any {
    fn clone_to_box(&self) -> Box<dyn RawStoreSignal>;
}

impl<T: 'static> RawStoreSignal for StoredSignal<T> {
    fn clone_to_box(&self) -> Box<dyn RawStoreSignal> {
        Box::new(self.clone())
    }
}

pub(crate) struct BoxedStoredSignal(Box<dyn RawStoreSignal>);

impl Clone for BoxedStoredSignal {
    fn clone(&self) -> Self {
        Self(self.0.clone_to_box())
    }
}

impl BoxedStoredSignal {
    pub fn downcast_ref<T: 'static>(&self) -> Option<&StoredSignal<T>> {
        (self.0.as_ref() as &dyn Any).downcast_ref()
    }
}

impl<T: 'static> From<StoredSignal<T>> for BoxedStoredSignal {
    fn from(value: StoredSignal<T>) -> Self {
        Self(Box::new(value))
    }
}
