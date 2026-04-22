use reactive_core::{Component, Signal};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Alignment {
    TopLeading,
    Top,
    TopTrailing,
    Leading,
    #[default]
    Center,
    Trailing,
    BottomLeading,
    Bottom,
    BottomTrailing,
}

impl Signal for Alignment {
    type Value = Self;

    fn read(&self) -> Self::Value {
        *self
    }
}

pub trait Stack: Component + Sized + 'static {
    fn new() -> Self;
    fn alignment(self, alignment: impl Signal<Value = Alignment> + 'static) -> Self;
    fn child(self, child: impl Component + 'static) -> Self;
}
