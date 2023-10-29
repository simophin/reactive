use crate::node::Node;

pub fn create_effect(func: impl Effect) {
    Node::with_current(|s| {
        s.expect("create_effect can be only called within the set up phase")
            .borrow_mut()
            .add_effect(func);
    });
}
pub trait Effect: 'static {
    fn run(&mut self) -> Box<dyn EffectCleanup>;
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
    F: FnOnce() + 'static,
{
    fn cleanup(&mut self) {
        self()
    }
}
