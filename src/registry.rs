use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap, HashSet},
    iter::once,
    rc::{Rc, Weak},
};

use crate::{
    effect::{Effect, EffectCleanup},
    node::{NodeRef, WeakNodeRef},
    tracker::Tracker,
    util::diff::{diff_sorted, DiffResult},
};

pub type SignalID = usize;
pub type EffectID = usize;

#[derive(Default)]
pub struct Registry {
    signal_id_seq: SignalID,
    effect_id_seq: EffectID,
    signals: HashMap<SignalID, SignalStateRef>,
    effects: HashMap<EffectID, EffectStateRef>,
}

pub type RegistryRef = Rc<RefCell<Registry>>;

impl Registry {
    pub fn add_signal(&mut self) -> SignalID {
        let signal_id = self.signal_id_seq;
        self.signal_id_seq = self.signal_id_seq.wrapping_add(1);

        self.signals.insert(
            signal_id,
            Rc::new(RefCell::new(SignalState {
                tracked_effects: Default::default(),
            })),
        );

        signal_id
    }

    pub fn get_signal(&self, signal_id: SignalID) -> Option<SignalStateRef> {
        self.signals.get(&signal_id).cloned()
    }

    pub fn remove_signal(&mut self, signal_id: SignalID) {
        let Some(state) = self.signals.remove(&signal_id) else {
            return;
        };

        let state = state.borrow();
        for effect_id in &state.tracked_effects {
            if let Some(effect) = self.effects.get(&effect_id) {
                effect.borrow_mut().tracking_signals.remove(&signal_id);
            }
        }
    }

    pub fn add_effect(
        &mut self,
        node: WeakNodeRef,
        effect: impl Effect,
    ) -> (EffectID, EffectStateRef) {
        let effect_id = self.effect_id_seq;
        self.effect_id_seq = self.effect_id_seq.wrapping_add(1);

        let state = Rc::new(RefCell::new(EffectState {
            id: effect_id,
            node,
            tracking_signals: Default::default(),
            func: Box::new(effect),
            clean_up: None,
        }));

        self.effects.insert(effect_id, state.clone());

        (effect_id, state)
    }

    pub fn remove_effect(&mut self, effect_id: EffectID) {
        let Some(state) = self.effects.remove(&effect_id) else {
            return;
        };

        let state = state.borrow();
        for signal_id in &state.tracking_signals {
            if let Some(signal) = self.signals.get(&signal_id) {
                signal.borrow_mut().tracked_effects.remove(&effect_id);
            }
        }
    }

    pub fn get_effects<'a>(
        &'a self,
        ids: impl Iterator<Item = EffectID> + 'a,
    ) -> impl Iterator<Item = EffectStateRef> + 'a {
        ids.filter_map(|id| self.effects.get(&id)).cloned()
    }

    pub fn apply_dependency_changes(
        &mut self,
        changes: impl Iterator<Item = (EffectID, DiffResult<SignalID>)>,
    ) {
        for (effect_id, diff) in changes {
            let effect = self.effects.get(&effect_id).expect("To have an effect");
            let mut effect = effect.borrow_mut();
            for signal_id in diff.removed {
                effect.tracking_signals.remove(&signal_id);
                if let Some(signal) = self.signals.get(&signal_id) {
                    signal.borrow_mut().tracked_effects.remove(&effect_id);
                }
            }

            for signal_id in diff.added {
                effect.tracking_signals.insert(signal_id);
                if let Some(signal) = self.signals.get(&signal_id) {
                    signal.borrow_mut().tracked_effects.insert(effect_id);
                }
            }
        }
    }
}

pub struct SignalState {
    pub tracked_effects: HashSet<EffectID>,
}

pub type SignalStateRef = Rc<RefCell<SignalState>>;

pub struct EffectState {
    pub id: EffectID,
    pub node: WeakNodeRef,
    pub tracking_signals: BTreeSet<SignalID>,
    pub func: Box<dyn Effect>,
    pub clean_up: Option<Box<dyn EffectCleanup>>,
}

pub type EffectStateRef = Rc<RefCell<EffectState>>;
pub type WeakEffectStateRef = Weak<RefCell<EffectState>>;

impl EffectState {
    pub fn run(&mut self) -> Option<DiffResult<SignalID>> {
        let Some(node) = self.node.upgrade() else {
            return None;
        };

        NodeRef::set_current(Some(node));
        if let Some(mut clean_up) = self.clean_up.take() {
            clean_up.cleanup();
        }
        Tracker::set_current(Some(Default::default()));
        self.clean_up.replace(self.func.run());
        let tracker = Tracker::set_current(None).expect("To have a tracker");
        NodeRef::set_current(None);

        Some(diff_sorted(
            self.tracking_signals.iter().cloned(),
            tracker.iter().cloned(),
        ))
    }
}
