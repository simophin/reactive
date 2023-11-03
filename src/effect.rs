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
    fn cleanup(self);
}

impl EffectCleanup for () {
    fn cleanup(self) {}
}

impl<F> EffectCleanup for F
where
    F: FnOnce() + 'static,
{
    fn cleanup(self) {
        self()
    }
}

impl<C: EffectCleanup> EffectCleanup for Option<C> {
    fn cleanup(self) {
        if let Some(mut c) = self {
            c.cleanup();
        }
    }
}
