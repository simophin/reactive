use crate::EffectContext;

pub trait Effect: 'static {
    fn run(&mut self, context: &mut EffectContext);
}

impl<F> Effect for F
where
    F: for<'a> FnMut(&'a mut EffectContext) + 'static,
{
    fn run(&mut self, context: &mut EffectContext) {
        self(context)
    }
}
