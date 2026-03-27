use crate::layout::CrossAxisAlignment;
use reactive_core::{Component, Signal};

pub trait Row: Component + Sized + 'static {
    fn new() -> Self;
    fn spacing(self, spacing: impl Signal<Value = usize> + 'static) -> Self;
    fn cross_axis_alignment(self, alignment: CrossAxisAlignment) -> Self;
    fn child(self, child: impl Component + 'static) -> Self;
}
