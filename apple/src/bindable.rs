use crate::{Prop, ViewBuilder};
use objc2::Message;
use reactive_core::Signal;

pub trait BindableView<V>: AsMut<ViewBuilder<V>> {
    fn bind<T>(
        mut self,
        props: &'static Prop<Self, V, T>,
        signal: impl Signal<Value = T> + 'static,
    ) -> Self
    where
        Self: Sized,
        V: Message,
    {
        self.as_mut().bind(props, signal);
        self
    }
}
