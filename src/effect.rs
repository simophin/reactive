use crate::node_ref::NodeRef;

pub fn create_effect(func: impl Effect) {
    NodeRef::with_current(|s| {
        s.expect("create_effect can be only called within the set up phase")
            .add_effect(func);
    });
}
pub trait Effect: 'static {
    fn run(&mut self) -> Box<dyn EffectCleanup>;
}

pub type BoxedEffect = Box<dyn Effect>;

impl Effect for BoxedEffect {
    fn run(&mut self) -> Box<dyn EffectCleanup> {
        self.as_mut().run()
    }
}

impl<F, C> Effect for F
where
    F: FnMut() -> C + 'static,
    C: EffectCleanup,
{
    fn run(&mut self) -> Box<dyn EffectCleanup> {
        Box::new(self())
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
