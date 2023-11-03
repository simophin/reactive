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

struct EffectFn<F>(F);

impl<F, C> Effect for EffectFn<F>
where
    F: FnMut(&mut EffectContext) -> C + 'static,
    C: EffectCleanup,
{
    type Cleanup = C;

    fn run(&mut self, ctx: &mut EffectContext) -> C {
        (self.0)(ctx)
    }
}

pub fn effect_fn<C: EffectCleanup>(
    f: impl FnMut(&mut EffectContext) -> C + 'static,
) -> impl Effect {
    EffectFn(f)
}
