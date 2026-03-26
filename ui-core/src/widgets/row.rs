use reactive_core::{Component, Signal};
use crate::layout::CrossAxisAlignment;

pub trait Row: Component + Sized + 'static {
    fn new() -> Self;
    fn spacing(self, spacing: impl Signal<Value = f64> + 'static) -> Self;
    fn cross_axis_alignment(self, alignment: CrossAxisAlignment) -> Self;
    fn child(self, child: impl Component + 'static) -> Self;
}
