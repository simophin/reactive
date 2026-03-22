use crate::component_scope::{BoxedEffectFn, ComponentId, Effect, EffectState};
use crate::signal::{Signal, StoredSignal};
use futures::{FutureExt, Stream, StreamExt};
use std::future::ready;

use super::ReactiveScope;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ResourceState<T> {
    Loading,
    Ready(T),
}

impl ReactiveScope {
    pub fn create_resource<I, T, F>(
        &mut self,
        component_id: ComponentId,
        input_signal: impl Signal<Value = I> + 'static,
        mut resource_fn: impl FnMut(I) -> F + 'static,
    ) -> impl Signal<Value = ResourceState<T>> + Copy + 'static
    where
        I: Clone + 'static,
        T: Clone + 'static,
        F: Future<Output = T> + 'static,
    {
        let signal = self.create_signal(ResourceState::<T>::Loading);
        let active_signal_tracker = self.active_signal_tracker.clone();

        let mut effect_fn: BoxedEffectFn = Box::new(move |_: &mut ReactiveScope| {
            let (input, signal_accessed) =
                active_signal_tracker.run_tracking(|| input_signal.read());

            EffectState {
                signal_accessed,
                pending_future: Some(Box::pin(resource_fn(input).map(move |result| {
                    signal.set_and_notify_changes(ResourceState::Ready(result));
                }))),
            }
        });

        let effect = Effect {
            effect_state: effect_fn(self),
            effect_fn,
        };

        if let Some(component) = self.components.get_mut(component_id) {
            component.effects.push(effect);
        }

        signal
    }

    pub fn create_stream<S, I, T>(
        &mut self,
        component_id: ComponentId,
        initial: T,
        input_signal: impl Signal<Value = I> + 'static,
        mut stream_producer: impl FnMut(I) -> S + 'static,
    ) -> impl Signal<Value = T> + Copy + 'static
    where
        I: Clone + 'static,
        T: Clone + 'static,
        S: Stream<Item = T> + 'static,
    {
        let signal = self.create_signal(initial);

        self.create_resource(component_id, input_signal, {
            move |input| {
                stream_producer(input).for_each(move |item| {
                    signal.set_and_notify_changes(item);
                    ready(())
                })
            }
        });

        signal
    }
}

#[cfg(test)]
mod tests {
    use crate::signal::Signal;
    use futures::task::noop_waker_ref;
    use std::sync::{Arc, Mutex};
    use std::task::Context;

    use crate::ResourceState;

    use super::ReactiveScope;

    #[test]
    fn test_resource() {
        let mut scope = ReactiveScope::default();
        let root = scope.create_child_component(None);

        let input_signal = scope.create_signal(42i32);
        let resource_signal =
            scope.create_resource(root, input_signal, |input| async move { input * 2 });

        assert_eq!(resource_signal.read(), ResourceState::Loading);

        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(resource_signal.read(), ResourceState::Ready(84));
    }

    #[test]
    fn test_resource_refires_on_input_change() {
        let mut scope = ReactiveScope::default();
        let root = scope.create_child_component(None);

        let input = scope.create_signal(1i32);
        let resource = scope.create_resource(root, input, |v| async move { v * 10 });

        // Initial load resolves in one tick (synchronous future)
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(resource.read(), ResourceState::Ready(10));

        // Changing the input re-fires the resource; with a sync future it resolves
        // in the same tick the effect re-runs — no intermediate Loading state
        input.update_if_changes(2);
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(resource.read(), ResourceState::Ready(20));
    }

    #[test]
    fn test_resource_dispose_during_loading() {
        let mut scope = ReactiveScope::default();
        let root = scope.create_child_component(None);
        let child = scope.create_child_component(Some(root));

        let input = scope.create_signal(());
        // Use a never-resolving future so the resource stays in Loading
        let resource = scope.create_resource(child, input, |_| futures::future::pending::<i32>());

        // First tick: effect fires, future is Pending
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(resource.read(), ResourceState::Loading);

        // Dispose while the future is still in-flight — it gets dropped with the component
        scope.dispose_component(child);

        // Tick — no effects, pending future is gone, signal untouched
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(resource.read(), ResourceState::Loading);
    }

    #[test]
    fn test_stream() {
        let mut scope = ReactiveScope::default();
        let root = scope.create_child_component(None);

        let signal =
            scope.create_stream(root, 0i32, || (), |_| futures::stream::iter(vec![1, 2, 3]));
        let result = Arc::new(Mutex::new(Vec::<i32>::new()));

        scope.create_effect(root, {
            let result = Arc::clone(&result);
            move |_, _: Option<()>| {
                result.lock().unwrap().push(signal.read());
            }
        });

        assert_eq!(*result.lock().unwrap(), vec![0]);

        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*result.lock().unwrap(), vec![0]);

        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*result.lock().unwrap(), vec![0, 3]);

        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*result.lock().unwrap(), vec![0, 3]);
    }

    #[test]
    fn test_stream_dispose() {
        let mut scope = ReactiveScope::default();
        let root = scope.create_child_component(None);
        let child = scope.create_child_component(Some(root));

        let signal =
            scope.create_stream(child, 0i32, || (), |_| futures::stream::iter(vec![1, 2, 3]));
        let result = Arc::new(Mutex::new(0i32));

        scope.create_effect(child, {
            let result = Arc::clone(&result);
            move |_, _: Option<()>| {
                *result.lock().unwrap() = signal.read();
            }
        });

        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*result.lock().unwrap(), 0);

        scope.dispose_component(child);

        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*result.lock().unwrap(), 0); // unchanged
    }
}
