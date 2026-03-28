use crate::layout::Alignment;
use reactive_core::{Component, Signal};

pub trait Stack: Component + Sized + 'static {
    fn new() -> Self;
    fn alignment(self, alignment: impl Signal<Value = Alignment> + 'static) -> Self;
    fn child(self, child: impl Component + 'static) -> Self;
}
