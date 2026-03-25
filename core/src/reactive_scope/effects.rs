use crate::component_scope::{BoxedEffectFn, ComponentId, Effect};
use crate::signal::{ReadSignal, StoredSignal};

use super::ReactiveScope;

impl ReactiveScope {
    pub(crate) fn create_effect<T: 'static>(
        &self,
        component_id: ComponentId,
        mut effect_fn: impl for<'a> FnMut(&ReactiveScope, Option<T>) -> T + 'static,
    ) {
        // Clone the tracker handle before dropping the borrow.
        let signal_tracker = self.0.borrow().active_signal_tracker.clone();
        let mut last_value = None;

        let mut effect_fn: BoxedEffectFn = Box::new(move |scope: &ReactiveScope| {
            let (value, signal_accessed) =
                signal_tracker.run_tracking(|| effect_fn(scope, std::mem::take(&mut last_value)));
            last_value.replace(value);
            (signal_accessed, None)
        });

        // Run once immediately — no borrow held, so the closure may freely call
        // any scope method without risking a RefCell panic.
        let (signal_accessed, in_flight) = effect_fn(self);
        let effect = Effect {
            effect_fn,
            signal_accessed,
            in_flight,
        };

        if let Some(component) = self.0.borrow_mut().components.get_mut(component_id) {
            component.push_effect(effect);
        }
    }

    pub(crate) fn create_memo<T: 'static>(
        &self,
        component_id: ComponentId,
        mut memo_fn: impl FnMut() -> T + 'static,
    ) -> ReadSignal<T> {
        let signal_tracker = self.0.borrow().active_signal_tracker.clone();
        let (initial_value, signal_accessed) = signal_tracker.run_tracking(|| memo_fn());

        let signal: StoredSignal<T> = self.create_signal(initial_value);
        let signal_tracker = self.0.borrow().active_signal_tracker.clone();

        let signal_for_effect = signal.clone();
        let effect = Effect {
            effect_fn: Box::new(move |_: &ReactiveScope| {
                let (value, signal_accessed) = signal_tracker.run_tracking(|| memo_fn());
                signal_for_effect.set_and_notify_changes(value);
                (signal_accessed, None)
            }),
            signal_accessed,
            in_flight: None,
        };

        if let Some(component) = self.0.borrow_mut().components.get_mut(component_id) {
            component.push_effect(effect);
        }

        ReadSignal(signal)
    }
}

#[cfg(test)]
mod tests {
    use crate::signal::Signal;
    use futures::task::noop_waker_ref;
    use std::sync::{Arc, Mutex};
    use std::task::Context;

    use super::ReactiveScope;

    #[test]
    fn test_signal_and_effect() {
        let scope = ReactiveScope::default();
        let root = scope.create_child_component(None);

        let count = scope.create_signal(0);
        let result = Arc::new(Mutex::new(0));

        scope.create_effect(root, {
            let count = count.clone();
            let result = Arc::clone(&result);
            move |_, last| {
                let count_value = count.read() + last.unwrap_or_default();
                *result.lock().unwrap() = count_value;
                count_value
            }
        });

        assert_eq!(*result.lock().unwrap(), 0);
        count.update_if_changes(1);
        assert_eq!(*result.lock().unwrap(), 0);

        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*result.lock().unwrap(), 1);

        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*result.lock().unwrap(), 1);

        count.update_if_changes(2);
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*result.lock().unwrap(), 3);
    }
}
