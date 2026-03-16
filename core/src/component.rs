use crate::component_scope::ComponentId;
use crate::{ContextKey, EffectContext, ReactiveScope, ResourceState, Signal};

pub trait Component {
    fn setup(self: Box<Self>, ctx: &mut SetupContext);
}

pub type BoxedComponent = Box<dyn Component>;

// ---------------------------------------------------------------------------
// Component implementations for common types
// ---------------------------------------------------------------------------

/// No-op component — renders nothing.
impl Component for () {
    fn setup(self: Box<Self>, _ctx: &mut SetupContext) {}
}

/// Function component — any `FnOnce(&mut SetupContext)` is a component.
impl<F: FnOnce(&mut SetupContext) + 'static> Component for F {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        (*self)(ctx);
    }
}

/// A list of components — each is set up as a child.
impl Component for Vec<BoxedComponent> {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        for child in *self {
            let mut child_ctx = ctx.new_child();
            child.setup(&mut child_ctx);
        }
    }
}

/// Scoped view for setting up a component.
/// Bundles a `ReactiveScope` reference with the component's ID,
/// so users never interact with these implementation details directly.
pub struct SetupContext<'a> {
    pub(crate) scope: &'a mut ReactiveScope,
    pub(crate) component_id: ComponentId,
}

impl<'a> SetupContext<'a> {
    pub fn new_root(scope: &'a mut ReactiveScope) -> Self {
        let root = scope.create_component(None);
        Self {
            scope,
            component_id: root,
        }
    }

    pub fn create_signal<T: 'static>(&mut self, initial: T) -> Signal<T> {
        self.scope.create_signal(initial)
    }

    pub fn read<T: Copy>(&self, signal: Signal<T>) -> T {
        self.scope.read(signal)
    }

    pub fn access<T, R>(&self, signal: Signal<T>, accessor: impl FnOnce(&T) -> R) -> R {
        self.scope.access(signal, accessor)
    }

    pub fn update<T: 'static>(&mut self, signal: Signal<T>, updater: impl FnOnce(&mut T) -> bool) {
        self.scope.update(signal, updater);
    }

    pub fn update_if_changed<T: PartialEq + 'static>(&mut self, signal: Signal<T>, new_value: T) {
        self.scope.update_if_changed(signal, new_value);
    }

    pub fn create_effect<T: 'static>(
        &mut self,
        effect_fn: impl for<'b> FnMut(&'b mut EffectContext<'_>, Option<&mut T>) -> T + 'static,
    ) {
        self.scope.create_effect(self.component_id, effect_fn);
    }

    pub fn create_resource<I: 'static, T: 'static, F: Future<Output = T> + 'static>(
        &mut self,
        input_fn: impl for<'b> FnMut(&'b mut EffectContext) -> I + 'static,
        resource_fn: impl FnMut(I) -> F + 'static,
    ) -> Signal<ResourceState<T>> {
        self.scope
            .create_resource(self.component_id, input_fn, resource_fn)
    }

    pub fn create_stream<T: 'static>(
        &mut self,
        initial: T,
        stream: impl futures::Stream<Item = T> + 'static,
    ) -> Signal<T> {
        self.scope
            .create_stream(self.component_id, initial, stream)
    }

    pub fn provide_context<T: 'static>(&mut self, key: &ContextKey<T>, value: T) {
        self.scope.provide_context(self.component_id, key, value);
    }

    pub fn use_context<T: 'static>(&self, key: &ContextKey<T>) -> Option<Signal<T>> {
        self.scope.use_context(self.component_id, key)
    }

    pub fn on_cleanup(&mut self, cleanup_fn: impl FnOnce() + 'static) {
        self.scope.on_cleanup(self.component_id, cleanup_fn);
    }

    pub fn new_child(&mut self) -> SetupContext<'_> {
        let child_id = self.scope.create_component(Some(self.component_id));
        SetupContext {
            scope: self.scope,
            component_id: child_id,
        }
    }

    pub fn component_id(&self) -> ComponentId {
        self.component_id
    }

    pub fn child(&mut self, component: impl Component + 'static) {
        let mut child_ctx = self.new_child();
        Box::new(component).setup(&mut child_ctx);
    }
}
