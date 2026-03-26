use reactive_core::{Component, Signal};
use std::error::Error;

pub trait Image: Component + Sized {
    type NativeHandle: TryFrom<Vec<u8>, Error = Box<dyn Error>> + Send + Sync + 'static;

    fn new(
        image: impl Signal<Value = Self::NativeHandle> + 'static,
        desc: Option<impl Signal<Value = String> + 'static>,
    ) -> Self;
}
