use crate::signal::{Signal, SignalID};
use std::any::Any;
use std::collections::BTreeMap;

type BoxedEffectFn = Box<dyn for<'a> FnMut(&'a mut EffectScope<'_>) -> ()>;

struct Effect {
    effect_fn: BoxedEffectFn,
    last_accessed_signals: Vec<SignalID>,
}

#[derive(Default)]
pub struct Scope {
    // Effects that must be run in next tick
    pending_effects_run: Vec<Effect>,

    // Effects that aren't going to be run in next tick. They will need somewhere to
    // be stored so that they can be re-run when their dependencies change.
    idle_effects: Vec<Effect>,

    signals: BTreeMap<SignalID, Box<dyn Any>>,
}

pub struct EffectScope<'a> {
    scope: &'a mut Scope,
    last_accessed_signals: &'a mut Vec<SignalID>,
}

impl Scope {
    pub fn create_effect(
        &mut self,
        effect_fn: impl for<'a> FnMut(&'a mut EffectScope<'_>) -> () + 'static,
    ) {
        let mut effect = Effect {
            effect_fn: Box::new(effect_fn),
            last_accessed_signals: Vec::new(),
        };

        (effect.effect_fn)(&mut EffectScope {
            scope: self,
            last_accessed_signals: &mut effect.last_accessed_signals,
        });

        self.idle_effects.push(effect);
    }

    pub fn create_signal<T: 'static>(&mut self, initial: T) -> Signal<T> {
        let signal_id = if let Some(entry) = self.signals.last_entry() {
            *entry.key() + 1
        } else {
            1
        };

        let signal = Signal::new(signal_id);
        self.signals.insert(signal_id, Box::new(initial));
        signal
    }

    pub(crate) fn run_pending_effects(&mut self) {
        for mut effect in std::mem::take(&mut self.pending_effects_run) {
            // Reuse the same last_accessed to save extra allocations
            effect.last_accessed_signals.clear();

            (effect.effect_fn)(&mut EffectScope {
                scope: self,
                last_accessed_signals: &mut effect.last_accessed_signals,
            });

            self.idle_effects.push(effect);
        }
    }

    pub fn update<T>(
        &mut self,
        signal: Signal<T>,
        updater: impl for<'b> FnOnce(&'b mut T) -> bool,
    ) {
        let value = self
            .signals
            .get_mut(&signal.id())
            .and_then(|signal| signal.downcast_mut::<T>())
            .expect("Signal not found or type mismatch");

        if updater(value) {
            // Check if any of the effects in idle_effects depend on this signal.
            // If they do, move them to pending_effects_run.
            for effect in std::mem::take(&mut self.idle_effects) {
                if effect.last_accessed_signals.contains(&signal.id()) {
                    self.pending_effects_run.push(effect);
                } else {
                    self.idle_effects.push(effect);
                }
            }
        }
    }

    pub fn update_if_changed<T: PartialEq>(&mut self, signal: Signal<T>, new_value: T) {
        self.update(signal, move |old_value| {
            if old_value != &new_value {
                *old_value = new_value;
                true
            } else {
                false
            }
        })
    }
}

impl<'a> EffectScope<'a> {
    pub fn access<T, R>(
        &mut self,
        signal: Signal<T>,
        accessor: impl for<'b> FnOnce(&'b T) -> R,
    ) -> R {
        let value = self
            .scope
            .signals
            .get(&signal.id())
            .and_then(|signal| signal.downcast_ref::<T>())
            .expect("Signal not found or type mismatch");

        let r = accessor(value);
        self.last_accessed_signals.push(signal.id());
        r
    }

    pub fn read<T>(&mut self, signal: Signal<T>) -> T
    where
        T: Copy,
    {
        self.access(signal, |value| *value)
    }

    pub fn update<T>(
        &mut self,
        signal: Signal<T>,
        updater: impl for<'b> FnOnce(&'b mut T) -> bool,
    ) {
        self.scope.update(signal, updater);
    }

    pub fn update_if_changed<T: PartialEq>(&mut self, signal: Signal<T>, new_value: T) {
        self.scope.update_if_changed(signal, new_value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_signal_and_effect() {
        let mut scope = Scope::default();
        let count = scope.create_signal(0);
        let result = Arc::new(Mutex::new(0));

        scope.create_effect({
            let result = Arc::clone(&result);
            move |scope| {
                let count_value = scope.read(count);
                *result.lock().unwrap() = count_value;
            }
        });

        assert_eq!(*result.lock().unwrap(), 0);
        scope.update_if_changed(count, 1);
        assert_eq!(*result.lock().unwrap(), 0);

        scope.run_pending_effects();
        assert_eq!(*result.lock().unwrap(), 1);
    }
}
