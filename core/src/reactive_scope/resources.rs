use crate::component_scope::{BoxedEffectFn, ComponentId, Effect, InFlightFuture};
use crate::signal::{ReadSignal, Signal};
use futures::{FutureExt, Stream, StreamExt};
use std::cell::RefCell;
use std::future::ready;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use super::ReactiveScope;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ResourceState<T> {
    /// The resource is loading. Contains the last successfully loaded value, if any.
    Loading(Option<T>),
    Ready(T),
}

impl ReactiveScope {
    pub(crate) fn create_resource<I, T, F>(
        &self,
        component_id: ComponentId,
        input_signal: impl Signal<Value = I> + 'static,
        mut resource_fn: impl FnMut(I) -> F + 'static,
    ) -> ReadSignal<ResourceState<T>>
    where
        I: 'static,
        T: Clone + 'static,
        F: Future<Output = T> + 'static,
    {
        let signal = self.create_signal(ResourceState::<T>::Loading(None));
        let active_signal_tracker = self.0.borrow().active_signal_tracker.clone();
        let last_ready: Rc<RefCell<Option<T>>> = Rc::new(RefCell::new(None));

        let mut effect_fn: BoxedEffectFn = Box::new(move |_: &ReactiveScope| {
            let (input, signal_accessed) =
                active_signal_tracker.run_tracking(|| input_signal.read());

            signal.set_and_notify_changes(ResourceState::Loading(last_ready.borrow().clone()));

            let last_ready = Rc::clone(&last_ready);
            let in_flight = Some(InFlightFuture {
                future: Box::pin(resource_fn(input).map(move |result| {
                    *last_ready.borrow_mut() = Some(result.clone());
                    signal.set_and_notify_changes(ResourceState::Ready(result));
                })),
                woken: Arc::new(AtomicBool::new(true)),
            });
            (signal_accessed, in_flight)
        });

        let (signal_accessed, in_flight) = effect_fn(self);
        let effect = Effect {
            effect_fn,
            signal_accessed,
            in_flight,
        };

        if let Some(component) = self.0.borrow_mut().components.get_mut(component_id) {
            component.push_effect(effect);
        }

        ReadSignal(signal)
    }

    pub(crate) fn create_stream<S, I, T>(
        &self,
        component_id: ComponentId,
        initial: T,
        input_signal: impl Signal<Value = I> + 'static,
        mut stream_producer: impl FnMut(I) -> S + 'static,
    ) -> ReadSignal<T>
    where
        I: 'static,
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

        ReadSignal(signal)
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
        let scope = ReactiveScope::default();
        let root = scope.create_child_component(None);

        let input_signal = scope.create_signal(42i32);
        let resource_signal =
            scope.create_resource(root, input_signal, |input| async move { input * 2 });

        assert_eq!(resource_signal.read(), ResourceState::Loading(None));

        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(resource_signal.read(), ResourceState::Ready(84));
    }

    #[test]
    fn test_resource_refires_on_input_change() {
        let scope = ReactiveScope::default();
        let root = scope.create_child_component(None);

        let input = scope.create_signal(1i32);
        let resource = scope.create_resource(root, input, |v| async move { v * 10 });

        // Initial load resolves in one tick (synchronous future)
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(resource.read(), ResourceState::Ready(10));

        // Changing the input re-fires the resource; the signal resets to Loading
        // then the sync future resolves in the same tick, ending at Ready.
        input.update_if_changes(2);
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(resource.read(), ResourceState::Ready(20));
    }

    #[test]
    fn test_resource_resets_to_loading_on_input_change() {
        let scope = ReactiveScope::default();
        let root = scope.create_child_component(None);

        let input = scope.create_signal(1i32);
        // First call resolves immediately; subsequent calls stay pending so we can
        // observe the intermediate Loading state.
        let call_count = std::cell::Cell::new(0usize);
        let resource = scope.create_resource(root, input, move |v| {
            let n = call_count.get();
            call_count.set(n + 1);
            if n == 0 {
                Box::pin(std::future::ready(v * 10))
                    as std::pin::Pin<Box<dyn std::future::Future<Output = i32>>>
            } else {
                Box::pin(std::future::pending::<i32>())
            }
        });

        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(resource.read(), ResourceState::Ready(10));

        // After input changes the signal must immediately reset to Loading, carrying
        // the last ready value so callers can show stale data while fetching.
        input.update_if_changes(2);
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(resource.read(), ResourceState::Loading(Some(10)));
    }

    #[test]
    fn test_resource_dispose_during_loading() {
        let scope = ReactiveScope::default();
        let root = scope.create_child_component(None);
        let child = scope.create_child_component(Some(root));

        let input = scope.create_signal(());
        // Use a never-resolving future so the resource stays in Loading
        let resource = scope.create_resource(child, input, |_| futures::future::pending::<i32>());

        // First tick: effect fires, future is Pending
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(resource.read(), ResourceState::Loading(None));

        // Dispose while the future is still in-flight — it gets dropped with the component
        scope.dispose_component(child);

        // Tick — no effects, pending future is gone, signal untouched
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(resource.read(), ResourceState::Loading(None));
    }

    #[test]
    fn test_stream() {
        let scope = ReactiveScope::default();
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
        let scope = ReactiveScope::default();
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
