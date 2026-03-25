use crate::signal::stored::SignalId;
use crate::sorted_vec::SortedVec;
use std::cell::RefCell;
use std::rc::Rc;
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
        self.wake();
    }

    /// Fire the stored waker without marking any signal dirty.
    ///
    /// Use this to ensure a tick is scheduled after component-tree
    /// manipulation that happens outside an existing reactive effect
    /// (e.g. inside an AppKit data-source callback).
    pub(super) fn wake(&self) {
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
}
