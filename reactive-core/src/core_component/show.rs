use derive_builder::Builder;

use crate::{
    component::{BoxedComponent, Component, ComponentFactory},
    setup_context::SetupContext,
};

#[derive(Builder)]
#[builder(pattern = "owned")]
pub struct Show<F, CS, CF> {
    test: F,
    success: CS,
    fail: CF,
}

impl<F, CS, CF> Component for Show<F, CS, CF>
where
    F: FnMut() -> bool + 'static,
    CS: ComponentFactory,
    CF: ComponentFactory,
{
    fn setup(mut self: Box<Self>, ctx: &mut SetupContext) {
        let mut last_success = None;

        ctx.create_effect(move |ctx| {
            let new_success = (self.test)();
            log::debug!("ShowEffect: new_success={new_success}, last={last_success:?}");
            match (last_success, new_success) {
                (Some(last), new) if last == new => return,
                _ => {}
            }

            last_success.replace(new_success);

            let child: BoxedComponent = if new_success {
                Box::new(self.success.create())
            } else {
                Box::new(self.fail.create())
            };

            let child = ctx.mount_node(child);

            ctx.with_current_node(move |node| {
                node.children.clear();
                node.children.push(child);
            });
        });
    }
}
