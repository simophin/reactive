use crate::component_scope::{ComponentId, ComponentScope, ContextKey, Effect, EffectState};
use crate::signal::{Signal, SignalId, StoredSignal};
use crate::sorted_vec::SortedVec;
use futures::StreamExt;
use futures::{FutureExt, Stream};
use slotmap::SlotMap;
use std::any::Any;
use std::cell::RefCell;
use std::future::ready;
use std::ops::Deref;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ResourceState<T> {
    Loading,
    Ready(T),
}

#[derive(Clone, Default)]
pub(crate) struct DirtySignalSet(Rc<RefCell<SortedVec<SignalId>>>);

impl DirtySignalSet {
    pub fn mark_dirty(&self, signal_id: SignalId) {
        self.0.borrow_mut().insert(signal_id);
    }

    pub fn clear(&self) {
        self.0.borrow_mut().clear();
    }

    pub fn borrow(&self) -> impl Deref<Target = SortedVec<SignalId>> {
        self.0.borrow()
    }
}

#[derive(Clone, Default)]
pub(crate) struct ActiveSignalTracker {
    active_tracking: Rc<RefCell<Option<SortedVec<SignalId>>>>,
}

impl ActiveSignalTracker {
    pub fn on_accessed(&self, signal_id: SignalId) {
        if let Some(tracking) = self.active_tracking.borrow_mut().as_mut() {
            tracking.insert(signal_id);
        }
    }

    pub fn run_tracking<T>(&self, f: impl FnOnce() -> T) -> (T, SortedVec<SignalId>) {
        assert!(
            self.active_tracking.borrow().is_none(),
            "Nested active tracking is not supported"
        );
        self.active_tracking.replace(Some(Default::default()));
        let result = f();
        let accessed = self.active_tracking.borrow_mut().take().unwrap_or_default();
        (result, accessed)
    }
}

#[derive(Default)]
pub struct ReactiveScope {
    components: SlotMap<ComponentId, ComponentScope>,
    dirty_signals: DirtySignalSet,
    active_signal_tracker: ActiveSignalTracker,
}

// ---------------------------------------------------------------------------
// Signal operations
// ---------------------------------------------------------------------------

impl ReactiveScope {
    pub fn create_signal<T: 'static>(&mut self, initial: T) -> StoredSignal<T> {
        StoredSignal::new(
            initial,
            self.dirty_signals.clone(),
            self.active_signal_tracker.clone(),
        )
    }
}

// ---------------------------------------------------------------------------
// Component operations
// ---------------------------------------------------------------------------

impl ReactiveScope {
    pub fn create_component(&mut self, parent: Option<ComponentId>) -> ComponentId {
        let context = parent
            .and_then(|p| self.components.get(p))
            .map(|p| Rc::clone(&p.context))
            .unwrap_or_default();

        let mut component = ComponentScope::new(parent);
        component.context = context;

        let id = self.components.insert(component);

        if let Some(parent_id) = parent {
            if let Some(parent) = self.components.get_mut(parent_id) {
                parent.children.push(id);
            }
        }

        id
    }

    pub fn dispose_component(&mut self, id: ComponentId) {
        let Some(component) = self.components.remove(id) else {
            return;
        };

        // Remove from parent's children list
        if let Some(parent_id) = component.parent {
            if let Some(parent) = self.components.get_mut(parent_id) {
                parent.children.retain(|&c| c != id);
            }
        }

        // Dispose children depth-first
        for child_id in component.children {
            self.dispose_component(child_id);
        }

        // Run cleanup functions
        for cleanup_fn in component.cleanup {
            cleanup_fn();
        }
    }

    pub fn on_cleanup(&mut self, component_id: ComponentId, cleanup_fn: impl FnOnce() + 'static) {
        if let Some(component) = self.components.get_mut(component_id) {
            component.cleanup.push(Box::new(cleanup_fn));
        }
    }
}

// ---------------------------------------------------------------------------
// Context operations
// ---------------------------------------------------------------------------

impl ReactiveScope {
    pub fn provide_context<T: 'static>(
        &mut self,
        component_id: ComponentId,
        key: &ContextKey<T>,
        initial_value: T,
    ) -> StoredSignal<T> {
        let signal = self.create_signal(initial_value);
        if let Some(component) = self.components.get_mut(component_id) {
            Rc::make_mut(&mut component.context).insert(key.id(), signal.clone().into());
        }

        signal
    }

    pub fn use_context<T: 'static>(
        &self,
        component_id: ComponentId,
        key: &ContextKey<T>,
    ) -> Option<impl Signal<Value = T> + Clone + 'static> {
        self.components
            .get(component_id)
            .and_then(|c| c.context.get(&key.id()))
            .and_then(|signal| signal.downcast_ref().cloned())
    }
}

// ---------------------------------------------------------------------------
// Effect operations
// ---------------------------------------------------------------------------

impl ReactiveScope {
    pub fn create_effect<T: 'static>(
        &mut self,
        component_id: ComponentId,
        mut effect_fn: impl for<'a> FnMut(&mut ReactiveScope, Option<T>) -> T + 'static,
    ) {
        let signal_tracker = self.active_signal_tracker.clone();

        let (initial_value, signal_accessed) =
            signal_tracker.run_tracking(|| effect_fn(self, None));

        let mut last_value = Some(initial_value);

        let effect = Effect {
            effect_fn: Box::new(move |scope| {
                let (value, signal_accessed) = signal_tracker
                    .run_tracking(|| effect_fn(scope, std::mem::take(&mut last_value)));

                last_value.replace(value);
                EffectState {
                    signal_accessed,
                    pending_future: None,
                }
            }),
            effect_state: EffectState {
                signal_accessed,
                pending_future: None,
            },
        };

        if let Some(component) = self.components.get_mut(component_id) {
            component.effects.push(effect);
        }
    }

    pub fn create_memo<T: PartialEq + Clone + 'static>(
        &mut self,
        component_id: ComponentId,
        mut memo_fn: impl FnMut() -> T + 'static,
    ) -> impl Signal<Value = T> + Clone + 'static {
        let (initial_value, signal_accessed) =
            self.active_signal_tracker.run_tracking(|| memo_fn());

        let signal = self.create_signal(initial_value);
        let signal_tracker = self.active_signal_tracker.clone();

        let effect = Effect {
            effect_fn: Box::new({
                let signal = signal.clone();
                move |_| {
                    let (value, signal_accessed) = signal_tracker.run_tracking(|| memo_fn());
                    signal.update_if_changes(value);
                    EffectState {
                        signal_accessed,
                        pending_future: None,
                    }
                }
            }),
            effect_state: EffectState {
                signal_accessed,
                pending_future: None,
            },
        };

        if let Some(component) = self.components.get_mut(component_id) {
            component.effects.push(effect);
        }

        signal
    }
}

// ---------------------------------------------------------------------------
// Resource operations
// ---------------------------------------------------------------------------

impl ReactiveScope {
    pub fn create_resource<I, T, F>(
        &mut self,
        component_id: ComponentId,
        input_signal: impl Signal<Value = I> + 'static,
        mut resource_fn: impl FnMut(I) -> F + 'static,
    ) -> impl Signal<Value = ResourceState<T>> + Clone + 'static
    where
        I: Clone + 'static,
        T: 'static,
        F: Future<Output = T> + 'static,
    {
        let signal = self.create_signal(ResourceState::<T>::Loading);
        let (initial_input, deps) = self
            .active_signal_tracker
            .run_tracking(|| input_signal.cloned());

        let mut produce_future = {
            let signal = signal.clone();
            move |input: I| {
                let signal = signal.clone();
                let future: Pin<Box<dyn Future<Output = ()>>> =
                    Box::pin(resource_fn(input).map(move |result| {
                        signal.set_and_notify_changes(ResourceState::Ready(result));
                        ()
                    }));

                Some(future)
            }
        };

        let pending_future = produce_future(initial_input);

        let active_signal_tracker = self.active_signal_tracker.clone();
        let effect = Effect {
            effect_fn: Box::new(move |_| {
                let (input, signal_accessed) =
                    active_signal_tracker.run_tracking(|| input_signal.cloned());
                let pending_future = produce_future(input);

                EffectState {
                    signal_accessed,
                    pending_future,
                }
            }),
            effect_state: EffectState {
                signal_accessed: deps,
                pending_future,
            },
        };

        if let Some(component) = self.components.get_mut(component_id) {
            component.effects.push(effect);
        }

        signal
    }
}

// ---------------------------------------------------------------------------
// Stream operations
// ---------------------------------------------------------------------------

impl ReactiveScope {
    pub fn create_stream<S, I, T>(
        &mut self,
        component_id: ComponentId,
        initial: T,
        input_signal: impl Signal<Value = I> + 'static,
        mut stream_producer: impl FnMut(I) -> S + 'static,
    ) -> impl Signal<Value = T> + Clone + 'static
    where
        I: Clone + 'static,
        T: 'static,
        S: Stream<Item = T> + 'static,
    {
        let signal = self.create_signal(initial);

        self.create_resource(component_id, input_signal, {
            let signal = signal.clone();
            move |input| {
                let signal = signal.clone();
                stream_producer(input).for_each(move |item| {
                    signal.set_and_notify_changes(item);
                    ready(())
                })
            }
        });

        signal
    }
}

// ---------------------------------------------------------------------------
// Tick
// ---------------------------------------------------------------------------

impl ReactiveScope {
    pub fn tick(&mut self, future_ctx: &mut Context) -> bool {
        struct EffectUpdate {
            component_id: ComponentId,
            effect: Effect,
            dirty: bool,
        }

        let mut updates = Vec::new();
        for (component_id, c) in &mut self.components {
            for effect in std::mem::take(&mut c.effects) {
                let dirty = effect
                    .effect_state
                    .signal_accessed
                    .intersects(&*self.dirty_signals.borrow());
                if dirty || effect.effect_state.pending_future.is_some() {
                    updates.push(EffectUpdate {
                        component_id,
                        effect,
                        dirty,
                    })
                } else {
                    c.effects.push(effect);
                }
            }
        }

        let mut has_more_dirty_effects = false;

        for mut update in updates {
            if update.dirty {
                update.effect.effect_state = (update.effect.effect_fn)(self);
            }

            has_more_dirty_effects |= update
                .effect
                .effect_state
                .signal_accessed
                .intersects(&*self.dirty_signals.borrow());

            if let Some(mut fut) = std::mem::take(&mut update.effect.effect_state.pending_future) {
                if let Poll::Pending = fut.as_mut().poll(future_ctx) {
                    update.effect.effect_state.pending_future.replace(fut);
                };
            }

            if let Some(c) = self.components.get_mut(update.component_id) {
                c.effects.push(update.effect);
            }
        }

        has_more_dirty_effects
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use futures::task::noop_waker_ref;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_signal_and_effect() {
        let mut scope = ReactiveScope::default();
        let root = scope.create_component(None);

        let count = scope.create_signal(0);
        let result = Arc::new(Mutex::new(0));

        scope.create_effect(root, {
            let result = Arc::clone(&result);
            let count = count.clone();
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

    #[test]
    fn test_resource() {
        let mut scope = ReactiveScope::default();
        let root = scope.create_component(None);

        let input_signal = scope.create_signal(42i32);

        let resource_signal =
            scope.create_resource(root, input_signal.clone(), |input| async move { input * 2 });

        assert_eq!(resource_signal.read(), ResourceState::Loading);

        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(resource_signal.read(), ResourceState::Ready(84));
    }

    #[test]
    fn test_component_dispose() {
        let mut scope = ReactiveScope::default();
        let root = scope.create_component(None);
        let child = scope.create_component(Some(root));

        let count = scope.create_signal(0);
        let result = Arc::new(Mutex::new(0));

        scope.create_effect(child, {
            let result = Arc::clone(&result);
            move |ctx, _: Option<&mut ()>| {
                *result.lock().unwrap() = ctx.read(count);
            }
        });

        assert_eq!(*result.lock().unwrap(), 0);

        scope.update_if_changed(count, 5);
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*result.lock().unwrap(), 5);

        // Dispose the child — effect should no longer run
        scope.dispose_component(child);

        scope.update_if_changed(count, 10);
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*result.lock().unwrap(), 5); // unchanged
    }

    #[test]
    fn test_context() {
        static THEME: ContextKey<&str> = ContextKey::new();

        let mut scope = ReactiveScope::default();
        let root = scope.create_component(None);

        scope.provide_context(root, &THEME, "dark");

        let child = scope.create_component(Some(root));
        let theme_signal = scope.use_context::<&str>(child, &THEME).unwrap();

        assert_eq!(scope.read(theme_signal), "dark");
    }

    #[test]
    fn test_context_override() {
        static THEME: ContextKey<&str> = ContextKey::new();

        let mut scope = ReactiveScope::default();
        let root = scope.create_component(None);
        scope.provide_context(root, &THEME, "dark");

        // Child overrides the context
        let child = scope.create_component(Some(root));
        scope.provide_context(child, &THEME, "light");

        // Grandchild under the overriding child sees "light"
        let grandchild = scope.create_component(Some(child));
        let gc_theme = scope.use_context::<&str>(grandchild, &THEME).unwrap();
        assert_eq!(scope.read(gc_theme), "light");

        // Sibling of child still sees the root's "dark"
        let sibling = scope.create_component(Some(root));
        let sibling_theme = scope.use_context::<&str>(sibling, &THEME).unwrap();
        assert_eq!(scope.read(sibling_theme), "dark");

        // Root itself still sees "dark"
        let root_theme = scope.use_context::<&str>(root, &THEME).unwrap();
        assert_eq!(scope.read(root_theme), "dark");
    }

    #[test]
    fn test_stream() {
        let mut scope = ReactiveScope::default();
        let root = scope.create_component(None);

        let stream = futures::stream::iter(vec![1, 2, 3]);
        let signal = scope.create_stream(root, 0i32, stream);
        let result = Arc::new(Mutex::new(Vec::<i32>::new()));

        scope.create_effect(root, {
            let result = Arc::clone(&result);
            move |ctx, _: Option<&mut ()>| {
                result.lock().unwrap().push(ctx.read(signal));
            }
        });

        // Initial effect run sees 0
        assert_eq!(*result.lock().unwrap(), vec![0]);

        // Each tick polls one item from the stream
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*result.lock().unwrap(), vec![0, 1]);

        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*result.lock().unwrap(), vec![0, 1, 2]);

        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*result.lock().unwrap(), vec![0, 1, 2, 3]);

        // Stream ended — no more updates
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*result.lock().unwrap(), vec![0, 1, 2, 3]);
    }

    #[test]
    fn test_stream_dispose() {
        let mut scope = ReactiveScope::default();
        let root = scope.create_component(None);
        let child = scope.create_component(Some(root));

        let stream = futures::stream::iter(vec![1, 2, 3]);
        let signal = scope.create_stream(child, 0i32, stream);
        let result = Arc::new(Mutex::new(0i32));

        scope.create_effect(child, {
            let result = Arc::clone(&result);
            move |ctx, _: Option<&mut ()>| {
                *result.lock().unwrap() = ctx.read(signal);
            }
        });

        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*result.lock().unwrap(), 1);

        // Dispose — stream and effect should stop
        scope.dispose_component(child);

        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*result.lock().unwrap(), 1); // unchanged
    }
}
