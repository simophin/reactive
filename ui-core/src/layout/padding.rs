use reactive_core::{Component, SetupContext, Signal};

use super::{BoxModifier, EdgeInsets, with_appended_box_modifier};

/// Insets a child by the given edge amounts.
pub struct Padding<I: Signal<Value = EdgeInsets>, C> {
    pub insets: I,
    pub child: C,
}

impl<I: Signal<Value = EdgeInsets> + 'static, C: Component + 'static> Component for Padding<I, C> {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let Padding { insets, child } = *self;
        with_appended_box_modifier(ctx, BoxModifier::Padding(insets.read()));
        ctx.boxed_child(Box::new(child));
    }
}
