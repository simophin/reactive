use crate::widgets::modifier::Modifier;
use crate::widgets::platform_view::PlatformBaseView;
use reactive_core::{Component, ContextKey, ReactiveScope, SetupContext, Signal};
use std::rc::Rc;

pub struct NativeView<C, U> {
    pub create: C,
    pub on_update: U,
    pub modifier: Modifier,
}

pub trait NativeViewRegistry {
    fn update_platform_view(&self, view: &dyn PlatformBaseView, modifier: Modifier);
    fn clear_platform_view(&self, view: &dyn PlatformBaseView);
}

pub static NATIVE_VIEW_REGISTRY: ContextKey<Rc<dyn NativeViewRegistry>> = ContextKey::new();

impl<C, U, N> NativeView<C, U>
where
    C: FnOnce() -> N,
    U: for<'a> FnMut(&'a mut N, &'a ReactiveScope) + 'static,
    N: PlatformBaseView + Clone,
{
    pub fn setup_in_component(self, ctx: &mut SetupContext) -> N {
        let Self {
            create,
            mut on_update,
            modifier,
        } = self;

        let native_view = create();
        let registry = ctx.use_context(&NATIVE_VIEW_REGISTRY);

        if let Some(registry) = registry.read() {
            registry.update_platform_view(&native_view, modifier);
        }

        {
            let mut native_view = native_view.clone();
            ctx.create_effect(move |scope, _| {
                on_update(&mut native_view, scope);
            });
        }

        if let Some(registry) = registry {
            let native_view = native_view.clone();
            ctx.on_cleanup(move || registry.read().clear_platform_view(&native_view));
        }

        native_view
    }
}

impl<C, U, N> Component for NativeView<C, U>
where
    C: FnOnce() -> N,
    U: FnMut(&mut N, &ReactiveScope) + 'static,
    N: PlatformBaseView + Clone,
{
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        (*self).setup_in_component(ctx);
    }
}
