use crate::widgets::WithModifier;
use reactive_core::{Component, Signal};

pub trait Button: Component + WithModifier + Sized + 'static {
    fn new(title: impl Signal<Value = String> + 'static, on_click: impl Fn() + 'static) -> Self;

    fn enabled(self, value: impl Signal<Value = bool> + 'static) -> Self;
}
