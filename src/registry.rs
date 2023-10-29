use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap, HashSet},
    rc::Rc,
};

use crate::{
    tracker::Tracker,
    util::diff::{diff_sorted, DiffResult},
};

pub type SignalID = usize;
pub type EffectID = usize;

thread_local! {
    static CURRENT_REGISTRY: RefCell<Option<Rc<RefCell<Registry>>>> = RefCell::new(None);
}

pub struct Registry {
    signal_id_seq: SignalID,
    effect_id_seq: EffectID,
    signals: HashMap<SignalID, RefCell<SignalState>>,
    effects: HashMap<EffectID, RefCell<EffectState>>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            signal_id_seq: 0,
            effect_id_seq: 0,
            signals: Default::default(),
            effects: Default::default(),
        }
    }

    pub fn set_current(registry: Option<Self>) -> Option<Rc<RefCell<Self>>> {
        CURRENT_REGISTRY.with(move |cell| {
            if let Some(registry) = registry {
                cell.borrow_mut().replace(Rc::new(RefCell::new(registry)))
            } else {
                cell.borrow_mut().take()
            }
        })
    }

    pub fn current() -> Rc<RefCell<Self>> {
        CURRENT_REGISTRY.with(|cell| {
            cell.borrow()
                .as_ref()
                .expect("Registry::current() is only accessible during mounting")
                .clone()
        })
    }

    pub fn notify_signal_changed(&mut self, id: SignalID) {
        let mut changes: Vec<(EffectID, DiffResult<SignalID>)> = Default::default();

        if let Some(state) = self.signals.get_mut(&id) {
            let state = state.borrow();

            for effect_id in state.tracked_effects.iter().cloned() {
                if let Some(effect) = self.effects.get(&effect_id) {
                    Tracker::set_current(Some(Tracker::default()));
                    (effect.borrow_mut().func)();
                    let tracker =
                        Tracker::set_current(None).expect("Tracker to have been set before");

                    changes.push((
                        effect_id,
                        diff_sorted(
                            tracker.iter().cloned(),
                            effect.borrow().tracking_signals.iter().cloned(),
                        ),
                    ));

                    effect.borrow_mut().tracking_signals = tracker.into_inner();
                }
            }
        }

        for (effect_id, diff) in changes {
            self.apply_effect_tracking_changes(effect_id, diff);
        }
    }

    pub fn call_all_effects(&mut self) {
        let mut changes: Vec<(EffectID, DiffResult<SignalID>)> = Default::default();

        for (id, effect) in &self.effects {
            Tracker::set_current(Some(Tracker::default()));
            (effect.borrow_mut().func)();
            let tracker = Tracker::set_current(None).expect("Tracker to have been set before");

            changes.push((
                *id,
                diff_sorted(
                    effect.borrow().tracking_signals.iter().cloned(),
                    tracker.iter().cloned(),
                ),
            ));

            effect.borrow_mut().tracking_signals = tracker.into_inner();
        }

        for (effect_id, diff) in changes {
            self.apply_effect_tracking_changes(effect_id, diff);
        }
    }

    fn apply_effect_tracking_changes(
        &mut self,
        effect_id: EffectID,
        DiffResult { added, removed }: DiffResult<SignalID>,
    ) {
        for signal_id in added {
            if let Some(signal) = self.signals.get(&signal_id) {
                signal.borrow_mut().tracked_effects.insert(effect_id);
            }
        }

        for signal_id in removed {
            if let Some(signal) = self.signals.get(&signal_id) {
                signal.borrow_mut().tracked_effects.remove(&effect_id);
            }
        }
    }

    pub fn register_signal(&mut self) -> SignalID {
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

    pub fn unregister_signal(&mut self, signal_id: SignalID) {
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

    pub fn unregister_effect(&mut self, effect_id: EffectID) {
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

    pub fn register_effect(&mut self, effect: impl FnMut() + 'static) -> EffectID {
        let effect_id = self.effect_id_seq;
        self.effect_id_seq = self.effect_id_seq.wrapping_add(1);

        self.effects.insert(
            effect_id,
            RefCell::new(EffectState {
                tracking_signals: Default::default(),
                func: Box::new(effect),
            }),
        );

        effect_id
    }
}

struct SignalState {
    tracked_effects: HashSet<EffectID>,
}

struct EffectState {
    tracking_signals: BTreeSet<SignalID>,
    func: Box<dyn FnMut() + 'static>,
}
