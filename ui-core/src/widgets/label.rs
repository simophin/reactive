use crate::layout::types::TextAlignment;
use reactive_core::{Component, Signal};

pub trait Label: Component + Sized + 'static {
    fn new(text: impl Signal<Value = String> + 'static) -> Self;
    fn font_size(self, size: impl Signal<Value = f64> + 'static) -> Self;
    fn alignment(self, alignment: impl Signal<Value = TextAlignment> + 'static) -> Self;
}
