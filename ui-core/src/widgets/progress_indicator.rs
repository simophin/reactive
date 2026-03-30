use reactive_core::{Component, Signal};

pub trait ProgressIndicator: Component + Sized + 'static {
    fn new_bar(value: impl Signal<Value = usize> + 'static) -> Self;
    fn new_spinner() -> Self;
}
