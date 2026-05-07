use reactive_core::{BoxedComponent, Component, Signal};
use std::marker::PhantomData;

pub struct CommonWindow<N> {
    pub(crate) title: Box<dyn Signal<Value = String>>,
    pub(crate) child: BoxedComponent,
    pub(crate) phantom_data: PhantomData<N>,
    pub(crate) initial_size: (f32, f32),
}

pub trait Window: Component + Sized + 'static {
    fn new(
        title: impl Signal<Value = String> + 'static,
        child: impl Component + 'static,
        width: f32,
        height: f32,
    ) -> Self;
}

impl<N> Window for CommonWindow<N>
where
    Self: Component + Sized + 'static,
{
    fn new(
        title: impl Signal<Value = String> + 'static,
        child: impl Component + 'static,
        width: f32,
        height: f32,
    ) -> Self {
        Self {
            title: Box::new(title),
            child: Box::new(child),
            initial_size: (width, height),
            phantom_data: Default::default(),
        }
    }
}
