use std::ops::Range;

use reactive_core::{Component, Signal};

pub trait Slider: Component + Sized + 'static {
    fn new(
        value: impl Signal<Value = usize> + 'static,
        range: impl Signal<Value = Range<usize>> + 'static,
        on_change: impl Fn(usize) + 'static,
    ) -> Self;
}
