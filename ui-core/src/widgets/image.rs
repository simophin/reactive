use reactive_core::{Component, Signal};

pub trait Image: Component + Sized + 'static {
    type NativeHandle;

    fn new<S: Into<String>>(
        image: impl Signal<Value = Self::NativeHandle> + 'static,
        desc: Option<impl Signal<Value = S> + 'static>,
    ) -> Self;
}
