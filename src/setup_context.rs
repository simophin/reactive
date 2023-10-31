use std::sync::atomic::{AtomicUsize, Ordering};

use async_broadcast::Sender;

use crate::{
    clean_up::{BoxedCleanUp, CleanUp},
    component::BoxedComponent,
    effect::{BoxedEffect, Effect},
    react_context::{SignalID, SignalNotifier},
    signal::{signal_pair, SignalReader, SignalWriter},
};

pub struct SetupContext {
    pub signal_change_sender: Sender<SignalID>,
    pub effects: Vec<BoxedEffect>,
    pub signals: Vec<SignalID>,
    pub clean_ups: Vec<BoxedCleanUp>,
    pub children: Vec<BoxedComponent>,
}

static SIGNAL_ID_SEQ: AtomicUsize = AtomicUsize::new(0);

impl SetupContext {
    pub fn new(signal_change_sender: Sender<SignalID>) -> Self {
        Self {
            signal_change_sender,
            effects: Default::default(),
            signals: Default::default(),
            clean_ups: Default::default(),
            children: Default::default(),
        }
    }
}

impl SetupContext {
    pub fn create_effect<'a, 'b>(&'a mut self, effect: impl Effect + 'b) {
        self.effects.push(Box::new(effect));
    }

    pub fn create_signal<T: 'static>(
        &mut self,
        initial_value: T,
    ) -> (SignalReader<T>, SignalWriter<T>) {
        let id = SIGNAL_ID_SEQ.fetch_add(1, Ordering::SeqCst);
        self.signals.push(id);

        signal_pair(
            id,
            initial_value,
            SignalNotifier::new(id, self.signal_change_sender.clone()),
        )
    }

    pub fn on_clean_up(&mut self, clean_up: impl CleanUp) {
        self.clean_ups.push(Box::new(clean_up));
    }
}
