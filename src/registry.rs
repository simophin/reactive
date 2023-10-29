use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap, HashSet},
    rc::Rc,
};

use crate::effect::{Effect, EffectCleanup};

pub type SignalID = usize;
pub type EffectID = usize;

pub struct Registry {
    signal_id_seq: SignalID,
    effect_id_seq: EffectID,
    signals: HashMap<SignalID, RefCell<SignalState>>,
    effects: HashMap<EffectID, RefCell<EffectState>>,
}

pub type RegistryRef = Rc<RefCell<Registry>>;

impl Registry {
    pub fn new() -> Self {
        Self {
            signal_id_seq: 0,
            effect_id_seq: 0,
            signals: Default::default(),
            effects: Default::default(),
        }
    }

    pub fn add_signal(&mut self) -> SignalID {
        let signal_id = self.signal_id_seq;
        self.signal_id_seq = self.signal_id_seq.wrapping_add(1);

        self.signals.insert(
            signal_id,
            RefCell::new(SignalState {
                tracked_effects: Default::default(),
            }),
        );

        signal_id
    }

    pub fn remove_signal(&mut self, signal_id: SignalID) {
        let Some(state) = self.signals.remove(&signal_id) else {
            return;
        };

        let state = state.into_inner();
        for effect_id in state.tracked_effects {
            if let Some(effect) = self.effects.get(&effect_id) {
                effect.borrow_mut().tracking_signals.remove(&signal_id);
            }
        }
    }

    pub fn add_effect(&mut self, effect: impl Effect) -> EffectID {
        let effect_id = self.effect_id_seq;
        self.effect_id_seq = self.effect_id_seq.wrapping_add(1);

        self.effects.insert(
            effect_id,
            RefCell::new(EffectState {
                tracking_signals: Default::default(),
                func: Box::new(effect),
                clean_up: None,
            }),
        );

        effect_id
    }

    pub fn remove_effect(&mut self, effect_id: EffectID) {
        let Some(state) = self.effects.remove(&effect_id) else {
            return;
        };

        let state = state.into_inner();
        for signal_id in state.tracking_signals {
            if let Some(signal) = self.signals.get(&signal_id) {
                signal.borrow_mut().tracked_effects.remove(&effect_id);
            }
        }
    }
}

struct SignalState {
    tracked_effects: HashSet<EffectID>,
}

struct EffectState {
    tracking_signals: BTreeSet<SignalID>,
    func: Box<dyn Effect>,
    clean_up: Option<Box<dyn EffectCleanup>>,
}
