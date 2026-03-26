use reactive_core::{Component, Signal};

pub trait TextInput: Component + Sized + 'static {
    fn new(
        value: impl Signal<Value = String> + 'static,
        on_change: impl Fn(&str) + 'static,
    ) -> Self;

    fn font_size(self, size: impl Signal<Value = f64> + 'static) -> Self;
}
