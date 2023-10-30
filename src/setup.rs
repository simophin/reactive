use std::{cell::RefCell, sync::atomic::AtomicUsize};

use crate::{
    clean_up::BoxedCleanUp, component::BoxedComponent, effect::BoxedEffect, registry::SignalID,
};

#[derive(Default)]
pub struct SetupContext {
    pub effects: Vec<BoxedEffect>,
    pub signals: Vec<SignalID>,
    pub clean_ups: Vec<BoxedCleanUp>,
    pub children: Vec<BoxedComponent>,
}

thread_local! {
    static CURRENT: RefCell<Option<SetupContext>> = RefCell::new(None);
}

static SIGNAL_ID_SEQ: AtomicUsize = AtomicUsize::new(0);

impl SetupContext {
    pub fn set_current(c: Option<Self>) -> Option<Self> {
        CURRENT.with(|current| current.replace(c))
    }

    pub fn with_current<R>(f: impl FnOnce(&mut Self) -> R) -> R {
        CURRENT.with(|current| {
            f(current
                .borrow_mut()
                .as_mut()
                .expect("To have a current context"))
        })
    }
}

impl SetupContext {
    pub fn add_effect(&mut self, effect: BoxedEffect) {
        self.effects.push(effect);
    }

    pub fn add_signal(&mut self) -> SignalID {
        let id = SIGNAL_ID_SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.signals.push(id);
        id
    }

    pub fn add_clean_up(&mut self, clean_up: BoxedCleanUp) {
        self.clean_ups.push(clean_up);
    }
}
