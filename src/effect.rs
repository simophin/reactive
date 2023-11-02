use crate::{effect_context::EffectContext, task::TaskHandle};

pub trait Effect: 'static {
    fn run(&mut self, ctx: &mut EffectContext) -> Box<dyn EffectCleanup>;
}

pub type BoxedEffect = Box<dyn Effect>;

impl Effect for BoxedEffect {
    fn run(&mut self, ctx: &mut EffectContext) -> Box<dyn EffectCleanup> {
        self.as_mut().run(ctx)
    }
}

impl<F, C> Effect for F
where
    F: for<'a> FnMut(&'a mut EffectContext) -> C + 'static,
    C: EffectCleanup,
{
    fn run(&mut self, ctx: &mut EffectContext) -> Box<dyn EffectCleanup> {
        Box::new(self(ctx))
    }
}

pub trait EffectCleanup: 'static {
    fn cleanup(&mut self);
}

impl EffectCleanup for () {
    fn cleanup(&mut self) {}
}

impl EffectCleanup for Option<TaskHandle> {
    fn cleanup(&mut self) {}
}

impl<F> EffectCleanup for Option<F>
where
    F: FnOnce() + 'static,
{
    fn cleanup(&mut self) {
        if let Some(f) = self.take() {
            f();
        }
    }
}
