use crate::component_scope::{
    BoxedEffectFn, ComponentId, ComponentScope, ContextKey, Effect, EffectState,
};
use crate::signal::{Signal, SignalId, StoredSignal};
use crate::sorted_vec::SortedVec;
use crate::vec_utils::extract_if;
use futures::StreamExt;
use futures::{FutureExt, Stream};
use slotmap::SlotMap;
use std::cell::RefCell;
use std::future::ready;
use std::ops::Deref;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ResourceState<T> {
    Loading,
    Ready(T),
}

#[derive(Clone, Default)]
pub(crate) struct DirtySignalSet {
    signals: Rc<RefCell<SortedVec<SignalId>>>,
    waker: Rc<RefCell<Option<Waker>>>,
}

impl DirtySignalSet {
    pub fn mark_dirty(&self, signal_id: SignalId) {
        self.signals.borrow_mut().insert(signal_id);
        if let Some(waker) = self.waker.borrow().as_ref() {
            waker.wake_by_ref();
        }
    }

    pub fn set_waker(&self, waker: Waker) {
        *self.waker.borrow_mut() = Some(waker);
    }

    pub fn borrow(&self) -> impl Deref<Target = SortedVec<SignalId>> {
        self.signals.borrow()
    }

    pub fn take(&self) -> SortedVec<SignalId> {
        std::mem::take(&mut *self.signals.borrow_mut())
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
    root: Vec<ComponentId>,

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
    pub fn create_child_component(&mut self, parent: Option<ComponentId>) -> ComponentId {
        let mut component = ComponentScope::default();
        component.parent = parent;
        let component_id = self.components.insert(component);
        match parent.and_then(|p| self.components.get_mut(p)) {
            Some(parent) => parent.children.push(component_id),
            None => self.root.push(component_id),
        }

        component_id
    }

    pub fn dispose_component(&mut self, id: ComponentId) {
        let Some(mut component) = self.components.remove(id) else {
            return;
        };

        // Clean up children first
        for child_id in std::mem::take(&mut component.children) {
            self.dispose_component(child_id);
        }

        // Run cleanup functions
        for cleanup_fn in component.cleanup {
            cleanup_fn();
        }
    }

    pub fn dispose_all_children(&mut self, id: ComponentId) {
        if let Some(component) = self.components.get_mut(id) {
            for child_id in std::mem::take(&mut component.children) {
                self.dispose_component(child_id);
            }
        }
    }

    pub fn dispose_children(&mut self, id: ComponentId, f: impl FnMut(&ComponentId) -> bool) {
        let to_dispose = self
            .components
            .get_mut(id)
            .map(move |scope| extract_if(&mut scope.children, f));

        if let Some(to_dispose) = to_dispose {
            for child in to_dispose {
                self.dispose_component(child);
            }
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
    pub fn provide_context<T: Clone + 'static>(
        &mut self,
        component_id: ComponentId,
        key: &'static ContextKey<T>,
        initial_value: T,
    ) -> StoredSignal<T> {
        let signal = self.create_signal(initial_value);
        if let Some(component) = self.components.get_mut(component_id) {
            component.context.insert(key.id(), signal.clone().into());
        }

        signal
    }

    pub fn use_context<T: Clone + 'static>(
        &self,
        component_id: ComponentId,
        key: &'static ContextKey<T>,
    ) -> Option<impl Signal<Value = T> + Clone + 'static> {
        let mut scope = self.components.get(component_id);

        while let Some(component) = scope {
            if let Some(signal) = component.context.get(&key.id()) {
                return signal.downcast_ref().cloned();
            }

            scope = component.parent.and_then(|id| self.components.get(id));
        }

        None
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
        let mut last_value = None;

        let mut effect_fn: BoxedEffectFn = Box::new(move |scope| {
            let (value, signal_accessed) =
                signal_tracker.run_tracking(|| effect_fn(scope, std::mem::take(&mut last_value)));

            last_value.replace(value);
            EffectState {
                signal_accessed,
                pending_future: None,
            }
        });

        let effect = Effect {
            effect_state: effect_fn(self),
            effect_fn,
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
        T: Clone + 'static,
        F: Future<Output = T> + 'static,
    {
        let signal = self.create_signal(ResourceState::<T>::Loading);

        let active_signal_tracker = self.active_signal_tracker.clone();
        let mut effect_fn: BoxedEffectFn = {
            let signal = signal.clone();
            Box::new(move |_| {
                let (input, signal_accessed) =
                    active_signal_tracker.run_tracking(|| input_signal.read());
                let signal = signal.clone();

                EffectState {
                    signal_accessed,
                    pending_future: Some(Box::pin(resource_fn(input).map(move |result| {
                        signal.set_and_notify_changes(ResourceState::Ready(result));
                        ()
                    }))),
                }
            })
        };

        let effect = Effect {
            effect_state: effect_fn(self),
            effect_fn,
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
        T: Clone + 'static,
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
    pub(crate) fn traverse_tree_depth_last(
        &mut self,
        start: ComponentId,
        f: &mut impl FnMut(ComponentId, &mut ComponentScope),
    ) {
        let Some(scope) = self.components.get_mut(start) else {
            return;
        };

        f(start, scope);

        let children = std::mem::take(&mut scope.children);
        for child in &children {
            self.traverse_tree_depth_last(*child, f);
        }

        if let Some(scope) = self.components.get_mut(start) {
            scope.children = children;
        }
    }

    pub fn tick(&mut self, future_ctx: &mut Context) {
        struct EffectUpdate {
            component_id: ComponentId,
            effect: Effect,
            dirty: bool,
        }

        // Store the waker so mark_dirty can schedule the next tick from anywhere.
        self.dirty_signals.set_waker(future_ctx.waker().clone());

        let mut updates = Vec::new();
        let root = std::mem::take(&mut self.root);
        let dirty_signal_set = self.dirty_signals.take();

        for component in &root {
            self.traverse_tree_depth_last(*component, &mut |component_id, c| {
                for effect in std::mem::take(&mut c.effects) {
                    let dirty = effect
                        .effect_state
                        .signal_accessed
                        .intersects(&dirty_signal_set);

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
            });
        }

        self.root = root;

        for mut update in updates {
            if update.dirty {
                // Any mark_dirty calls inside here will wake the waker directly.
                update.effect.effect_state = (update.effect.effect_fn)(self);
            }

            if let Some(mut fut) = std::mem::take(&mut update.effect.effect_state.pending_future) {
                if let Poll::Pending = fut.as_mut().poll(future_ctx) {
                    update.effect.effect_state.pending_future.replace(fut);
                };
            }

            if let Some(c) = self.components.get_mut(update.component_id) {
                c.effects.push(update.effect);
            }
        }
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
        let root = scope.create_child_component(None);

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
        let root = scope.create_child_component(None);

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
        let root = scope.create_child_component(None);
        let child = scope.create_child_component(Some(root));

        let count = scope.create_signal(0);
        let result = Arc::new(Mutex::new(0));

        scope.create_effect(child, {
            let result = Arc::clone(&result);
            let count = count.clone();
            move |_, _: Option<()>| {
                *result.lock().unwrap() = count.read();
            }
        });

        assert_eq!(*result.lock().unwrap(), 0);

        count.update_if_changes(5);
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*result.lock().unwrap(), 5);

        // Dispose the child — effect should no longer run
        scope.dispose_component(child);

        count.update_if_changes(10);
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*result.lock().unwrap(), 5); // unchanged
    }

    #[test]
    fn test_context() {
        static THEME: ContextKey<&str> = ContextKey::new();

        let mut scope = ReactiveScope::default();
        let root = scope.create_child_component(None);

        scope.provide_context(root, &THEME, "dark");

        let child = scope.create_child_component(Some(root));
        let theme_signal = scope.use_context::<&str>(child, &THEME).unwrap();

        assert_eq!(theme_signal.read(), "dark");
    }

    #[test]
    fn test_context_override() {
        static THEME: ContextKey<&str> = ContextKey::new();

        let mut scope = ReactiveScope::default();
        let root = scope.create_child_component(None);
        scope.provide_context(root, &THEME, "dark");

        // Child overrides the context
        let child = scope.create_child_component(Some(root));
        scope.provide_context(child, &THEME, "light");

        // Grandchild under the overriding child sees "light"
        let grandchild = scope.create_child_component(Some(child));
        let gc_theme = scope.use_context::<&str>(grandchild, &THEME).unwrap();
        assert_eq!(gc_theme.read(), "light");

        // Sibling of child still sees the root's "dark"
        let sibling = scope.create_child_component(Some(root));
        let sibling_theme = scope.use_context::<&str>(sibling, &THEME).unwrap();
        assert_eq!(sibling_theme.read(), "dark");

        // Root itself still sees "dark"
        let root_theme = scope.use_context::<&str>(root, &THEME).unwrap();
        assert_eq!(root_theme.read(), "dark");
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

        // Initial effect run sees 0
        assert_eq!(*result.lock().unwrap(), vec![0]);

        // Tick 1: resource future runs, for_each exhausts the sync iterator all at once (signal=3).
        // The user effect was not dirty at collection time so it doesn't run this tick.
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*result.lock().unwrap(), vec![0]);

        // Tick 2: user effect is now dirty (signal changed), runs once and sees the final value.
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*result.lock().unwrap(), vec![0, 3]);

        // Stream ended — no more updates
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

        // Tick 1: resource future exhausts the sync iterator (signal=3).
        // User effect was not dirty at collection time so it doesn't run yet.
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*result.lock().unwrap(), 0);

        // Dispose before the user effect gets a chance to run
        scope.dispose_component(child);

        // Tick 2: signal is dirty but component is disposed — effect does not run
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*result.lock().unwrap(), 0); // unchanged
    }
}
