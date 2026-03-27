use reactive_core::{Component, Signal};

pub trait Window: Component + Sized + 'static {
    fn new(
        title: impl Signal<Value = String> + 'static,
        child: impl Component + 'static,
        width: f64,
        height: f64,
    ) -> Self;
}
