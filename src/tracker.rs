use derive_more::{Deref, DerefMut};
use std::cell::RefCell;
use std::collections::BTreeSet;

use crate::react_context::SignalID;

thread_local! {
    static CURRENT_TRACKER: RefCell<Option<Tracker>> = RefCell::new(None);
}

#[derive(Default, Deref, DerefMut)]
pub struct Tracker(BTreeSet<SignalID>);

impl Tracker {
    pub fn into_inner(self) -> BTreeSet<SignalID> {
        self.0
    }

    pub fn set_current(tracker: Option<Self>) -> Option<Self> {
        CURRENT_TRACKER.with(move |cell| match tracker {
            Some(tracker) => cell.borrow_mut().replace(tracker),
            None => cell.borrow_mut().take(),
        })
    }

    pub fn track_signal(signal_id: SignalID) {
        CURRENT_TRACKER.with(|cell| {
            cell.borrow_mut()
                .as_mut()
                .expect("No current tracker")
                .0
                .insert(signal_id);
        });
    }
}
