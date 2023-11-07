use derive_builder::Builder;

use crate::{
    component::{BoxedComponent, Component, ComponentFactory},
    setup_context::SetupContext,
};

#[derive(Builder)]
#[builder(pattern = "owned")]
pub struct Show<T>
where
    T: FnMut() -> bool + 'static,
{
    test: T,
    #[builder(setter(into))]
    child: ComponentFactory,
    #[builder(setter(into), default = "ComponentFactory::empty()")]
    fallback: ComponentFactory,
}

impl<T> Component for Show<T>
where
    T: FnMut() -> bool + 'static,
{
    fn setup(mut self: Box<Self>, ctx: &mut SetupContext) {
        let mut last_success = None;

        ctx.create_effect_fn(move |ctx| {
            let new_success = (self.test)();
            log::debug!("ShowEffect: new_success={new_success}, last={last_success:?}");
            match (last_success, new_success) {
                (Some(last), new) if last == new => return,
                _ => {}
            }

            last_success.replace(new_success);

            let child: BoxedComponent = if new_success {
                self.child.create()
            } else {
                self.fallback.create()
            };

            let child = ctx.mount_node(child);

            ctx.with_current_node(move |node| {
                node.children.clear();
                node.children.push(child);
            });
        });
    }
}
