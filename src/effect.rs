use crate::effect_context::EffectContext;

pub trait Effect: 'static {
    type Cleanup: EffectCleanup;

    fn run(&mut self, ctx: &mut EffectContext) -> Self::Cleanup;
}

impl<F, C> Effect for F
where
    F: FnMut() -> C + 'static,
    C: EffectCleanup,
{
    type Cleanup = C;

    fn run(&mut self, _ctx: &mut EffectContext) -> C {
        self()
    }
}

pub trait EffectCleanup: 'static {
    fn cleanup(&mut self);
}

impl EffectCleanup for () {
    fn cleanup(&mut self) {}
}

impl<F> EffectCleanup for F
where
    F: FnMut() + 'static,
{
    fn cleanup(&mut self) {
        self()
    }
}

impl<C: EffectCleanup> EffectCleanup for Option<C> {
    fn cleanup(&mut self) {
        if let Some(mut c) = self.take() {
            c.cleanup();
        }
    }
}
