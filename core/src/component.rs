use crate::component_scope::ComponentId;
use crate::signal::{ReadSignal, StoredSignal};
use crate::{ContextKey, ReactiveScope, ResourceState, Signal};
use futures::Stream;

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
        let root = scope.create_child_component(None);
        Self {
            scope,
            component_id: root,
        }
    }

    pub fn create_signal<T: 'static>(&mut self, initial: T) -> StoredSignal<T> {
        self.scope.create_signal(initial)
    }

    pub fn create_effect<T: 'static>(
        &mut self,
        effect_fn: impl for<'b> FnMut(&'b mut ReactiveScope, Option<T>) -> T + 'static,
    ) {
        self.scope.create_effect(self.component_id, effect_fn);
    }

    pub fn create_memo<T: 'static>(
        &mut self,
        memo_fn: impl FnMut() -> T + 'static,
    ) -> ReadSignal<T> {
        self.scope.create_memo(self.component_id, memo_fn)
    }

    pub fn create_resource<I, T, F>(
        &mut self,
        input_fn: impl Signal<Value = I> + 'static,
        resource_fn: impl FnMut(I) -> F + 'static,
    ) -> ReadSignal<ResourceState<T>>
    where
        I: Clone + 'static,
        T: Clone + 'static,
        F: Future<Output = T> + 'static,
    {
        self.scope
            .create_resource(self.component_id, input_fn, resource_fn)
    }

    pub fn create_stream<S, I, T>(
        &mut self,
        initial: T,
        input_signal: impl Signal<Value = I> + 'static,
        stream_producer: impl FnMut(I) -> S + 'static,
    ) -> ReadSignal<T>
    where
        I: Clone + 'static,
        T: Clone + 'static,
        S: Stream<Item = T> + 'static,
    {
        self.scope
            .create_stream(self.component_id, initial, input_signal, stream_producer)
    }

    pub fn provide_context<T: Clone + 'static>(
        &mut self,
        key: &'static ContextKey<T>,
        value: T,
    ) -> StoredSignal<T> {
        self.scope.provide_context(self.component_id, key, value)
    }

    pub fn use_context<T: Clone + 'static>(
        &self,
        key: &'static ContextKey<T>,
    ) -> Option<ReadSignal<T>> {
        self.scope.use_context(self.component_id, key)
    }

    pub fn on_cleanup(&mut self, cleanup_fn: impl FnOnce() + 'static) {
        self.scope.on_cleanup(self.component_id, cleanup_fn);
    }

    pub fn new_child(&mut self) -> SetupContext<'_> {
        let child_id = self.scope.create_child_component(Some(self.component_id));
        SetupContext {
            scope: self.scope,
            component_id: child_id,
        }
    }

    pub fn component_id(&self) -> ComponentId {
        self.component_id
    }

    pub fn child(&mut self, component: impl Component + 'static) -> ComponentId {
        let mut child_ctx = self.new_child();
        Box::new(component).setup(&mut child_ctx);
        child_ctx.component_id
    }
}
