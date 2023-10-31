use crate::{
    component::{boxed_component, BoxedComponent, Component, ComponentFactory},
    effect_context::EffectContext,
    setup_context::SetupContext,
};

pub struct Show<F, CS, CF>(Option<ShowData<F, CS, CF>>);

struct ShowData<F, CS, CF> {
    test: F,
    success: CS,
    fail: CF,
}

impl<F, CS, CF> Show<F, CS, CF>
where
    F: FnMut() -> bool + 'static,
    CS: ComponentFactory,
    CF: ComponentFactory,
{
    pub fn new(test: F, success: CS, fail: CF) -> BoxedComponent {
        boxed_component(Self(Some(ShowData {
            test,
            success,
            fail,
        })))
    }
}

impl<F, CS, CF> Component for Show<F, CS, CF>
where
    F: FnMut() -> bool + 'static,
    CS: ComponentFactory,
    CF: ComponentFactory,
{
    fn setup(&mut self, ctx: &mut SetupContext) {
        let mut data = self
            .0
            .take()
            .expect("Setup called twice for Show component");

        ctx.create_effect(move |ctx: &mut EffectContext| {
            let child: BoxedComponent = if (data.test)() {
                Box::new(data.success.create())
            } else {
                Box::new(data.fail.create())
            };

            ctx.queue_children_replacement(vec![child])
        });
    }
}
