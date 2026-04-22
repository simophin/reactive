use crate::Prop;
use crate::widgets::WithModifier;
use crate::widgets::modifier::Modifier;
use reactive_core::{Component, ContextKey, ReactiveScope, SetupContext, Signal};
use std::rc::Rc;

pub struct NativeView<BN: 'static, N: 'static> {
    pub(crate) create: Box<dyn FnOnce(&mut SetupContext) -> N>,
    pub(crate) on_update: Box<dyn FnMut(&mut N, &ReactiveScope)>,
    pub(crate) modifier: Modifier,
    pub(crate) registry_key: &'static ContextKey<Rc<dyn NativeViewRegistry<BN>>>,
    pub(crate) to_base: fn(N) -> BN,
    pub(crate) prop_binders: Vec<Box<dyn FnOnce(N, &mut SetupContext)>>,
}

pub trait NativeViewRegistry<V> {
    fn update_view(&self, view: &V, modifier: Modifier);
    fn clear_view(&self, view: &V);
}

impl<BN, N> NativeView<BN, N>
where
    BN: Clone + 'static,
    N: Clone + 'static,
{
    pub fn setup_in_component(self, ctx: &mut SetupContext) -> N {
        let Self {
            create,
            mut on_update,
            modifier,
            registry_key,
            to_base,
            prop_binders,
        } = self;

        let native_view = create(ctx);
        let registry = ctx.use_context(registry_key);

        if let Some(registry) = registry.read() {
            let base_view = to_base(native_view.clone());
            registry.update_view(&base_view, modifier);
            ctx.on_cleanup(move || registry.clear_view(&base_view));
        }

        for binder in prop_binders {
            binder(native_view.clone(), ctx);
        }

        {
            let mut native_view = native_view.clone();
            ctx.create_effect(move |scope, _| {
                on_update(&mut native_view, scope);
            });
        }

        native_view
    }

    pub fn new(
        create: impl FnOnce(&mut SetupContext) -> N + 'static,
        to_base: fn(N) -> BN,
        on_update: impl FnMut(&mut N, &ReactiveScope) + 'static,
        modifier: Modifier,
        registry_key: &'static ContextKey<Rc<dyn NativeViewRegistry<BN>>>,
    ) -> Self {
        Self {
            create: Box::new(create),
            to_base,
            on_update: Box::new(on_update),
            modifier,
            registry_key,
            prop_binders: Vec::new(),
        }
    }

    pub fn bind<FrameworkType: 'static, ValueType: 'static>(
        mut self,
        prop: Prop<FrameworkType, N, ValueType>,
        value: impl Signal<Value = ValueType> + 'static,
    ) -> Self {
        self.prop_binders.push(Box::new(move |view: N, ctx| {
            prop.bind(ctx, view, value);
        }));

        self
    }
}

impl<BN, N> WithModifier for NativeView<BN, N>
where
    N: Clone + 'static,
    BN: Clone + 'static,
{
    fn modifier(mut self, modifier: Modifier) -> Self {
        self.modifier = modifier;
        self
    }
}

impl<BN, N> Component for NativeView<BN, N>
where
    N: Clone + 'static,
    BN: Clone + 'static,
{
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        (*self).setup_in_component(ctx);
    }
}
