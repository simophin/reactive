use crate::signal::SignalId;
use crate::sorted_vec::SortedVec;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::task::Waker;

// ---------------------------------------------------------------------------
// DirtySignalSet
// ---------------------------------------------------------------------------

#[derive(Clone, Default)]
pub(crate) struct DirtySignalSet {
    pub(super) signals: Rc<RefCell<SortedVec<SignalId>>>,
    pub(super) waker: Rc<RefCell<Option<Waker>>>,
}

impl DirtySignalSet {
    pub fn mark_dirty(&self, signal_id: SignalId) {
        self.signals.borrow_mut().insert(signal_id);
        if let Some(waker) = self.waker.borrow().as_ref() {
            waker.wake_by_ref();
        }
    }

    pub fn set_waker(&self, waker: Waker) {
        *self.waker.borrow_mut() = Some(waker);
    }

    pub fn take(&self) -> SortedVec<SignalId> {
        std::mem::take(&mut *self.signals.borrow_mut())
    }

    pub fn downgrade(&self) -> WeakDirtySignalSet {
        WeakDirtySignalSet {
            signals: Rc::downgrade(&self.signals),
            waker: Rc::downgrade(&self.waker),
        }
    }
}

pub(crate) struct WeakDirtySignalSet {
    signals: Weak<RefCell<SortedVec<SignalId>>>,
    waker: Weak<RefCell<Option<Waker>>>,
}

impl WeakDirtySignalSet {
    pub fn upgrade(&self) -> Option<DirtySignalSet> {
        Some(DirtySignalSet {
            signals: self.signals.upgrade()?,
            waker: self.waker.upgrade()?,
        })
    }
}

// ---------------------------------------------------------------------------
// ActiveSignalTracker
// ---------------------------------------------------------------------------

#[derive(Clone, Default)]
pub(crate) struct ActiveSignalTracker {
    pub(super) active_tracking: Rc<RefCell<Vec<SortedVec<SignalId>>>>,
}

impl ActiveSignalTracker {
    pub fn on_accessed(&self, signal_id: SignalId) {
        if let Some(tracking) = self.active_tracking.borrow_mut().last_mut() {
            tracking.insert(signal_id);
        }
    }

    pub fn run_tracking<T>(&self, f: impl FnOnce() -> T) -> (T, SortedVec<SignalId>) {
        self.active_tracking.borrow_mut().push(Default::default());
        let result = f();
        let accessed = self.active_tracking.borrow_mut().pop().unwrap();
        (result, accessed)
    }

    pub fn downgrade(&self) -> WeakActiveSignalTracker {
        WeakActiveSignalTracker {
            active_tracking: Rc::downgrade(&self.active_tracking),
        }
    }
}

pub(crate) struct WeakActiveSignalTracker {
    active_tracking: Weak<RefCell<Vec<SortedVec<SignalId>>>>,
}

impl WeakActiveSignalTracker {
    pub fn upgrade(&self) -> Option<ActiveSignalTracker> {
        Some(ActiveSignalTracker {
            active_tracking: self.active_tracking.upgrade()?,
        })
    }
}
