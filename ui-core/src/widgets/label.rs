use crate::widgets::WithModifier;
use reactive_core::{Component, Signal};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextAlignment {
    Leading,
    Center,
    Trailing,
}

pub trait Label: Component + WithModifier + Sized + 'static {
    fn new(text: impl Signal<Value = String> + 'static) -> Self;
    fn font_size(self, size: impl Signal<Value = f64> + 'static) -> Self;
    fn alignment(self, alignment: impl Signal<Value = TextAlignment> + 'static) -> Self;
}
