use crate::component_scope::{
    BoxedEffectFn, ComponentId, ComponentScope, ContextKey, Effect, Resource, ResourceProducerFn,
};
use crate::signal::{Signal, SignalId};
use crate::sorted_vec::SortedVec;
use crate::vec_utils::extract_if;
use futures::FutureExt;
use slotmap::SlotMap;
use std::any::Any;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ResourceState<T> {
    Loading,
    Ready(T),
}

pub struct ReactiveScope {
    signals: SlotMap<SignalId, Box<dyn Any>>,
    components: SlotMap<ComponentId, ComponentScope>,
    dirty_signals: SortedVec<SignalId>,
}

impl Default for ReactiveScope {
    fn default() -> Self {
        Self {
            signals: SlotMap::with_key(),
            components: SlotMap::with_key(),
            dirty_signals: SortedVec::default(),
        }
    }
}

/// Mutable view into the reactive scope, passed to effect closures.
/// Tracks which signals the effect reads for dependency tracking.
pub struct EffectContext<'a> {
    scope: &'a mut ReactiveScope,
    last_accessed_signals: &'a mut SortedVec<SignalId>,
}

// ---------------------------------------------------------------------------
// Signal operations
// ---------------------------------------------------------------------------

impl ReactiveScope {
    pub fn create_signal<T: 'static>(&mut self, initial: T) -> Signal<T> {
        let id = self.signals.insert(Box::new(initial));
        Signal::new(id)
    }

    pub fn access<T, R>(&self, signal: Signal<T>, accessor: impl FnOnce(&T) -> R) -> R {
        accessor(
            self.signals
                .get(signal.id())
                .and_then(|v| v.downcast_ref::<T>())
                .expect("Signal not found or type mismatch"),
        )
    }

    pub fn read<T: Copy>(&self, signal: Signal<T>) -> T {
        self.access(signal, |v| *v)
    }

    pub fn update<T: 'static>(&mut self, signal: Signal<T>, updater: impl FnOnce(&mut T) -> bool) {
        let value = self
            .signals
            .get_mut(signal.id())
            .and_then(|v| v.downcast_mut::<T>())
            .expect("Signal not found or type mismatch");

        if updater(value) {
            self.dirty_signals.insert(signal.id());
        }
    }

    pub fn update_if_changed<T: PartialEq + 'static>(&mut self, signal: Signal<T>, new_value: T) {
        self.update(signal, move |old| {
            if old != &new_value {
                *old = new_value;
                true
            } else {
                false
            }
        });
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

        // Remove resource signals
        for resource in &component.resources {
            self.signals.remove(resource.signal_id);
        }

        // Clean up context signals if this is the last reference
        if Rc::strong_count(&component.context) == 1 {
            for (_, &signal_id) in component.context.iter() {
                self.signals.remove(signal_id);
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
    pub fn provide_context<T: 'static>(
        &mut self,
        component_id: ComponentId,
        key: &ContextKey<T>,
        value: T,
    ) {
        let signal_id = self.signals.insert(Box::new(value));

        if let Some(component) = self.components.get_mut(component_id) {
            Rc::make_mut(&mut component.context).insert(key.id(), signal_id);
        }
    }

    pub fn use_context<T: 'static>(
        &self,
        component_id: ComponentId,
        key: &ContextKey<T>,
    ) -> Option<Signal<T>> {
        self.components
            .get(component_id)
            .and_then(|c| c.context.get(&key.id()))
            .map(|&id| Signal::new(id))
    }
}

// ---------------------------------------------------------------------------
// Effect operations
// ---------------------------------------------------------------------------

impl ReactiveScope {
    fn boxed_effect_fn<T: 'static>(
        mut effect_fn: impl for<'a> FnMut(&'a mut EffectContext<'_>, Option<&mut T>) -> T + 'static,
    ) -> BoxedEffectFn {
        Box::new(move |ctx, value| {
            Box::new(effect_fn(
                ctx,
                value.and_then(|v| v.downcast_mut::<T>()).as_deref_mut(),
            ))
        })
    }

    pub fn create_effect<T: 'static>(
        &mut self,
        component_id: ComponentId,
        effect_fn: impl for<'a> FnMut(&'a mut EffectContext<'_>, Option<&mut T>) -> T + 'static,
    ) {
        let mut last_accessed_signals = SortedVec::default();
        let mut effect_fn = Self::boxed_effect_fn(effect_fn);

        // Run immediately for initial value
        let last_value = effect_fn(
            &mut EffectContext {
                scope: self,
                last_accessed_signals: &mut last_accessed_signals,
            },
            None,
        );

        let effect = Effect {
            effect_fn,
            last_value,
            last_accessed_signals,
        };

        if let Some(component) = self.components.get_mut(component_id) {
            component.effects.push(effect);
        }
    }
}

// ---------------------------------------------------------------------------
// Resource operations
// ---------------------------------------------------------------------------

impl ReactiveScope {
    pub fn create_resource<I: 'static, T: 'static, F: Future<Output = T> + 'static>(
        &mut self,
        component_id: ComponentId,
        mut input_fn: impl for<'a> FnMut(&'a mut EffectContext) -> I + 'static,
        mut resource_fn: impl FnMut(I) -> F + 'static,
    ) -> Signal<ResourceState<T>> {
        let signal = self.create_signal(ResourceState::<T>::Loading);

        let mut producer: ResourceProducerFn = Box::new(move |ctx: &mut EffectContext| {
            Box::pin(
                resource_fn(input_fn(ctx))
                    .map(|result| Box::new(ResourceState::Ready(result)) as Box<dyn Any>),
            ) as Pin<Box<dyn Future<Output = Box<dyn Any>>>>
        });

        let mut deps = Default::default();
        let future = producer(&mut EffectContext {
            scope: self,
            last_accessed_signals: &mut deps,
        });

        let resource = Resource {
            signal_id: signal.id(),
            deps,
            producer,
            pending_future: Some(future),
        };

        if let Some(component) = self.components.get_mut(component_id) {
            component.resources.push(resource);
        }

        signal
    }
}

// ---------------------------------------------------------------------------
// Tick
// ---------------------------------------------------------------------------

impl ReactiveScope {
    pub fn tick(&mut self, future_ctx: &mut Context) -> bool {
        let dirty = std::mem::take(&mut self.dirty_signals);

        // For each component, extract dirty effects/resources, run them, then restore
        for comp_id in self.components.keys().collect::<Vec<_>>() {
            let comp = match self.components.get_mut(comp_id) {
                Some(c) => c,
                None => continue,
            };

            // Extract dirty effects out of the component
            let mut dirty_effects = extract_if(&mut comp.effects, |e| {
                e.last_accessed_signals.intersects(&dirty)
            });

            // Extract dirty resources out of the component
            let mut dirty_resources =
                extract_if(&mut comp.resources, |r| r.deps.intersects(&dirty));

            // Run dirty effects
            for effect in &mut dirty_effects {
                effect.last_accessed_signals.clear();
                let new_value = (effect.effect_fn)(
                    &mut EffectContext {
                        scope: self,
                        last_accessed_signals: &mut effect.last_accessed_signals,
                    },
                    Some(&mut effect.last_value),
                );
                effect.last_value = new_value;
            }

            // Re-run dirty resources
            for resource in &mut dirty_resources {
                resource.deps.clear();
                let future = (resource.producer)(&mut EffectContext {
                    scope: self,
                    last_accessed_signals: &mut resource.deps,
                });
                resource.pending_future = Some(future);
            }

            // Restore — component may have been removed during effect execution
            if let Some(comp) = self.components.get_mut(comp_id) {
                comp.effects.extend(dirty_effects);
                comp.resources.extend(dirty_resources);
            }
        }

        // Poll pending futures
        let has_pending = self.poll_pending_futures(future_ctx);

        has_pending || !self.dirty_signals.is_empty()
    }

    fn poll_pending_futures(&mut self, future_ctx: &mut Context) -> bool {
        let mut has_pending = false;

        for comp_id in self.components.keys().collect::<Vec<_>>() {
            let comp = match self.components.get_mut(comp_id) {
                Some(c) => c,
                None => continue,
            };

            // Extract resources with pending futures
            let mut pending = extract_if(&mut comp.resources, |r| r.pending_future.is_some());

            for resource in &mut pending {
                if let Some(future) = resource.pending_future.as_mut() {
                    match future.as_mut().poll(future_ctx) {
                        Poll::Ready(result) => {
                            resource.pending_future = None;
                            if let Some(slot) = self.signals.get_mut(resource.signal_id) {
                                *slot = result;
                            }
                            self.dirty_signals.insert(resource.signal_id);
                        }
                        Poll::Pending => {
                            has_pending = true;
                        }
                    }
                }
            }

            // Restore resources
            if let Some(comp) = self.components.get_mut(comp_id) {
                comp.resources.extend(pending);
            }
        }

        has_pending
    }
}

// ---------------------------------------------------------------------------
// EffectContext
// ---------------------------------------------------------------------------

impl<'a> EffectContext<'a> {
    pub fn access<T, R>(&mut self, signal: Signal<T>, accessor: impl FnOnce(&T) -> R) -> R {
        let r = self.scope.access(signal, accessor);
        self.last_accessed_signals.insert(signal.id());
        r
    }

    pub fn read<T: Copy>(&mut self, signal: Signal<T>) -> T {
        let r = self.scope.read(signal);
        self.last_accessed_signals.insert(signal.id());
        r
    }

    pub fn update<T: 'static>(&mut self, signal: Signal<T>, updater: impl FnOnce(&mut T) -> bool) {
        self.scope.update(signal, updater);
    }

    pub fn update_if_changed<T: PartialEq + 'static>(&mut self, signal: Signal<T>, new_value: T) {
        self.scope.update_if_changed(signal, new_value);
    }

    pub fn dispose_component(&mut self, id: ComponentId) {
        self.scope.dispose_component(id);
    }

    pub fn setup_child(&mut self, parent: ComponentId) -> crate::SetupContext<'_> {
        let child_id = self.scope.create_component(Some(parent));
        crate::SetupContext {
            scope: self.scope,
            component_id: child_id,
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
        let root = scope.create_component(None);

        let count = scope.create_signal(0);
        let result = Arc::new(Mutex::new(0));

        scope.create_effect(root, {
            let result = Arc::clone(&result);
            move |ctx, last: Option<&mut i32>| {
                let count_value = ctx.read(count) + last.cloned().unwrap_or_default();
                *result.lock().unwrap() = count_value;
                count_value
            }
        });

        assert_eq!(*result.lock().unwrap(), 0);
        scope.update_if_changed(count, 1);
        assert_eq!(*result.lock().unwrap(), 0);

        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*result.lock().unwrap(), 1);

        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*result.lock().unwrap(), 1);

        scope.update_if_changed(count, 2);
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*result.lock().unwrap(), 3);
    }

    #[test]
    fn test_resource() {
        let mut scope = ReactiveScope::default();
        let root = scope.create_component(None);

        let input_signal = scope.create_signal(42i32);

        let resource_signal = scope.create_resource(
            root,
            move |ctx| ctx.read(input_signal),
            |input| async move { input * 2 },
        );

        assert_eq!(scope.read(resource_signal), ResourceState::Loading);

        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(scope.read(resource_signal), ResourceState::Ready(84));
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
}
