use derive_more::{Deref, DerefMut};
use std::cell::RefCell;

use crate::{react_context::SignalID, util::signal_set::SignalSet};

thread_local! {
    static CURRENT_TRACKER: RefCell<Option<Tracker>> = RefCell::new(None);
}

#[derive(Default, Deref, DerefMut)]
pub struct Tracker(SignalSet);

impl Tracker {
    pub fn with_current<T>(self, f: impl FnOnce() -> T) -> (Self, T) {
        CURRENT_TRACKER.with(|cell| {
            cell.borrow_mut().replace(self);
            let r = f();
            (cell.borrow_mut().take().unwrap(), r)
        })
    }

    pub fn track_signal(signal_id: SignalID) {
        CURRENT_TRACKER.with(|cell| match cell.borrow_mut().as_mut() {
            Some(tracker) => tracker.insert(signal_id),
            None => {
                log::warn!("No current tracker, signal will be ignored");
            }
        });
    }
}
