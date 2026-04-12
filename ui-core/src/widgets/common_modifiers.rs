use crate::layout::EdgeInsets;
use crate::widgets::modifier::{Modifier, ModifierKey};
use reactive_core::Signal;

pub trait CommonModifiers {
    fn paddings(self, edge_insets: impl Signal<Value = EdgeInsets> + 'static) -> Self;
    fn get_paddings(&self) -> impl Signal<Value = Option<EdgeInsets>> + 'static;
}

static PADDINGS_KEY: ModifierKey<EdgeInsets> =
    ModifierKey::new(|old_signal, new_value| old_signal.read().plus(&new_value));

impl CommonModifiers for Modifier {
    fn paddings(self, edge_insets: impl Signal<Value = EdgeInsets> + 'static) -> Self {
        self.with(&PADDINGS_KEY, edge_insets)
    }

    fn get_paddings(&self) -> impl Signal<Value = Option<EdgeInsets>> + 'static {
        self.get(&PADDINGS_KEY)
    }
}
