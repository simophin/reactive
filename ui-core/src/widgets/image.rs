use reactive_core::{Component, Signal};
use std::error::Error;

pub trait Image: Component + Sized + 'static {
    type NativeHandle: for<'a> TryFrom<&'a [u8], Error = Box<dyn Error + Send + Sync>>
        + Clone
        + Eq
        + Send
        + Sync
        + 'static;

    fn new<S: Into<String>>(
        image: impl Signal<Value = Self::NativeHandle> + 'static,
        desc: Option<impl Signal<Value = S> + 'static>,
    ) -> Self;
}
