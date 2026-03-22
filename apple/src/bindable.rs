use crate::{Prop, ViewBuilder};
use objc2::Message;
use reactive_core::Signal;

pub trait BindableView<V> {
    fn get_builder(&mut self) -> &mut ViewBuilder<V>;

    fn bind<T>(
        mut self,
        props: &'static Prop<Self, V, T>,
        signal: impl Signal<Value = T> + 'static,
    ) -> Self
    where
        Self: Sized,
        V: Message,
    {
        self.get_builder().bind(props, signal);
        self
    }
}
