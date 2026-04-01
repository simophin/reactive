use reactive_core::{Component, SetupContext, Signal};

use super::{BoxModifier, EdgeInsets, with_appended_box_modifier};

/// Insets a child by the given edge amounts.
pub struct Padding<I: Signal, C> {
    pub insets: I,
    pub child: C,
}

impl<I, C> Component for Padding<I, C>
where
    I: Signal + 'static,
    C: Component + 'static,
    <I as Signal>::Value: Into<EdgeInsets> + 'static,
{
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let Padding { insets, child } = *self;
        with_appended_box_modifier(ctx, move || BoxModifier::Padding(insets.read().into()));
        ctx.boxed_child(Box::new(child));
    }
}
