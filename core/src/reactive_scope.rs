use crate::signal::{Signal, SignalID};
use futures::FutureExt;
use std::any::Any;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll};

pub type EffectId = u64;

type BoxedEffectFn = Box<dyn for<'a> FnMut(&'a mut EffectScope<'_>) -> ()>;

struct Effect {
    id: EffectId,
    effect_fn: BoxedEffectFn,
    last_accessed_signals: Vec<SignalID>,
}

struct PendingFuture {
    result_signal_id: SignalID,
    future: Pin<Box<dyn Future<Output = Box<dyn Any>>>>,
}

#[derive(Default)]
pub struct ReactiveScope {
    effect_id_seq: AtomicU64,

    // Effects that must be run in next tick
    pending_effects_run: Vec<Effect>,

    // Effects that aren't going to be run in next tick. They will need somewhere to
    // be stored so that they can be re-run when their dependencies change.
    idle_effects: Vec<Effect>,

    pending_futures: HashMap<EffectId, PendingFuture>,

    signals: BTreeMap<SignalID, Box<dyn Any>>,
}

pub struct EffectScope<'a> {
    effect_id: EffectId,
    scope: &'a mut ReactiveScope,
    last_accessed_signals: &'a mut Vec<SignalID>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ResourceState<T> {
    Loading,
    Ready(T),
}

impl ReactiveScope {
    pub fn create_effect(
        &mut self,
        effect_fn: impl for<'a> FnMut(&'a mut EffectScope<'_>) -> () + 'static,
    ) -> EffectId {
        let id = self.effect_id_seq.fetch_add(1, Ordering::Relaxed);
        let mut effect = Effect {
            id,
            effect_fn: Box::new(effect_fn),
            last_accessed_signals: Vec::new(),
        };

        (effect.effect_fn)(&mut EffectScope {
            effect_id: id,
            scope: self,
            last_accessed_signals: &mut effect.last_accessed_signals,
        });

        self.idle_effects.push(effect);
        id
    }

    pub fn create_resource<I: 'static, T: 'static, F: Future<Output = T> + 'static>(
        &mut self,
        mut input_fn: impl for<'a> FnMut(&'a mut EffectScope) -> I + 'static,
        mut resource_fn: impl FnMut(I) -> F + 'static,
    ) -> (Signal<ResourceState<T>>, EffectId) {
        let signal = self.create_signal(ResourceState::Loading);

        let effect_id = self.create_effect(move |scope| {
            let input = input_fn(scope);
            let future = Box::pin(
                resource_fn(input).map(move |o| Box::new(ResourceState::Ready(o)) as Box<dyn Any>),
            );

            scope.scope.pending_futures.insert(
                scope.effect_id,
                PendingFuture {
                    result_signal_id: signal.id(),
                    future,
                },
            );
        });

        (signal, effect_id)
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

    pub fn remove_effect(&mut self, effect_id: EffectId) {
        self.idle_effects.retain(|effect| effect.id != effect_id);
        self.pending_effects_run
            .retain(|effect| effect.id != effect_id);
    }

    // Run pending effects and poll pending futures.
    // Returns true if there are still pending effects or futures after the tick.
    pub(crate) fn tick(&mut self, future_ctx: &mut Context) -> bool {
        for mut effect in std::mem::take(&mut self.pending_effects_run) {
            // Reuse the same last_accessed to save extra allocations
            effect.last_accessed_signals.clear();

            (effect.effect_fn)(&mut EffectScope {
                effect_id: effect.id,
                scope: self,
                last_accessed_signals: &mut effect.last_accessed_signals,
            });

            self.idle_effects.push(effect);
        }

        let mut changed_signals = HashSet::new();

        // Run pending futures and update their result signals
        self.pending_futures
            .retain(|_, p| match p.future.poll_unpin(future_ctx) {
                Poll::Ready(r) => {
                    if let Some(signal) = self.signals.get_mut(&p.result_signal_id) {
                        *signal = r;
                        changed_signals.insert(p.result_signal_id);
                    }

                    false
                }

                Poll::Pending => true,
            });

        // Check if any signals have been changed by the completed futures.
        if !changed_signals.is_empty() {
            for effect in std::mem::take(&mut self.idle_effects) {
                if effect
                    .last_accessed_signals
                    .iter()
                    .any(|signal_id| changed_signals.contains(signal_id))
                {
                    self.pending_effects_run.push(effect);
                } else {
                    self.idle_effects.push(effect);
                }
            }
        }

        !self.pending_effects_run.is_empty() || !self.pending_futures.is_empty()
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

    pub fn access<T, R>(
        &mut self,
        signal: Signal<T>,
        accessor: impl for<'b> FnOnce(&'b T) -> R,
    ) -> R {
        let value = self
            .signals
            .get(&signal.id())
            .and_then(|signal| signal.downcast_ref::<T>())
            .expect("Signal not found or type mismatch");

        accessor(value)
    }

    pub fn read<T>(&mut self, signal: Signal<T>) -> T
    where
        T: Copy,
    {
        self.access(signal, |value| *value)
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
        let r = self.scope.access(signal, accessor);
        self.last_accessed_signals.push(signal.id());
        r
    }

    pub fn read<T>(&mut self, signal: Signal<T>) -> T
    where
        T: Copy,
    {
        self.scope.read(signal)
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
    use futures::task::noop_waker_ref;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_signal_and_effect() {
        let mut scope = ReactiveScope::default();
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

        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*result.lock().unwrap(), 1);
    }

    #[test]
    fn test_resource() {
        let mut scope = ReactiveScope::default();
        let input_signal = scope.create_signal(42i32);

        let (resource_signal, _effect_id) = scope.create_resource(
            move |scope| scope.read(input_signal),
            |input| async move { input * 2 },
        );

        // Effect ran during creation, queueing a future. Resource is still Loading.
        assert_eq!(scope.read(resource_signal), ResourceState::Loading);

        // Tick: polls the future (ready immediately) and updates the signal.
        scope.tick(&mut Context::from_waker(noop_waker_ref()));

        // Resource signal should now be Ready.
        assert_eq!(scope.read(resource_signal), ResourceState::Ready(84));
    }
}
