use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap, HashSet, VecDeque},
    task::Waker,
};

use linked_hash_set::LinkedHashSet;

use crate::{
    effect::{BoxedEffect, EffectCleanup},
    registry::{EffectID, SignalID},
    task::Task,
    tracker::Tracker,
    util::diff::{diff_sorted, DiffResult},
};

#[derive(Default)]
pub struct ReactiveContext {
    effect_id_seq: EffectID,
    effects: HashMap<EffectID, EffectState>,
    signal_deps: HashMap<SignalID, LinkedHashSet<EffectID>>,
    waker: Option<Waker>,
    pending_tasks: VecDeque<Task>,
    pending_effect_runs: LinkedHashSet<EffectID>,
}

thread_local! {
    static CURRENT: RefCell<Option<ReactiveContext>> = Default::default();
}

impl ReactiveContext {
    pub fn set_current(context: Option<ReactiveContext>) -> Option<ReactiveContext> {
        CURRENT.with(|current| current.replace(context))
    }

    pub fn with_current<T>(f: impl FnOnce(&mut ReactiveContext) -> T) -> T {
        CURRENT.with(move |current| {
            let mut current = current.borrow_mut();
            f(current
                .as_mut()
                .expect("To have reactive context set before"))
        })
    }
}

impl ReactiveContext {
    pub fn set_waker(&mut self, waker: &Waker) {
        self.waker.replace(waker.clone());
    }

    pub fn new_effect(&mut self, effect: BoxedEffect) -> EffectID {
        let id = self.effect_id_seq;
        self.effect_id_seq += 1;
        self.effects.insert(
            id,
            EffectState {
                id,
                effect,
                last_clean_up: None,
                last_tracked_signals: Default::default(),
            },
        );
        id
    }

    pub fn remove_effect(&mut self, id: EffectID) {
        self.pending_effect_runs.remove(&id);
        let Some(mut effect) = self.effects.remove(&id) else {
            return;
        };

        for signal in effect.last_tracked_signals {
            self.remove_signal_dep(id, signal);
        }
    }

    pub fn push_task(&mut self, task: Task) {
        self.pending_tasks.push_back(task);
        self.waker.wake_by_ref();
    }

    pub fn pop_task(&mut self) -> Option<Task> {
        self.pending_tasks.pop_front()
    }

    pub fn run_pending_effects(&mut self) {
        for id in self.pending_effect_runs.iter() {
            let effect = self.effects.get_mut(id).expect("To have effect");
            let diff = effect.run();
            self.update_signal_deps(*id, diff);
        }
    }

    pub fn schedule_effect_run(&mut self, id: EffectID) {
        self.pending_effect_runs.insert(id);
        self.waker.wake_by_ref();
    }

    pub fn notify_signal_read(&mut self, signal: SignalID) {
        if let Some(deps) = self.signal_deps.get(&signal) {
            for id in deps.iter().cloned() {
                self.pending_effect_runs.insert(id);
            }

            if !deps.is_empty() {
                self.waker.wake_by_ref();
            }
        }
    }

    pub fn update_signal_deps(&mut self, id: EffectID, diff: DiffResult<SignalID>) {
        let DiffResult { added, removed } = diff;

        for signal_id in added {
            self.signal_deps.entry(signal_id).or_default().insert(id);
        }

        for signal_id in removed {
            self.remove_signal_dep(id, signal_id);
        }
    }

    fn remove_signal_dep(&mut self, effect: EffectID, signal: SignalID) {
        if let Some(deps) = self.signal_deps.get_mut(&signal) {
            deps.remove(&effect);

            if deps.is_empty() {
                self.signal_deps.remove(&signal);
            }
        }
    }
}

pub struct EffectState {
    pub id: EffectID,
    pub effect: BoxedEffect,
    pub last_clean_up: Option<Box<dyn EffectCleanup>>,
    pub last_tracked_signals: BTreeSet<SignalID>,
}

impl EffectState {
    fn run(&mut self) -> DiffResult<SignalID> {
        if let Some(mut clean_up) = self.last_clean_up.take() {
            clean_up.cleanup();
        }

        Tracker::set_current(Some(Default::default()));
        self.last_clean_up.replace(self.effect.run());
        let tracker = Tracker::set_current(None).expect("To have tracker set before");

        let result = diff_sorted(
            self.last_tracked_signals.iter().cloned(),
            tracker.iter().cloned(),
        );

        self.last_tracked_signals = tracker.into_inner();
        result
    }
}

impl Drop for EffectState {
    fn drop(&mut self) {
        if let Some(mut clean_up) = self.last_clean_up.take() {
            clean_up.cleanup();
        }
    }
}
