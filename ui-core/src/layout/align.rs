use reactive_core::{Component, SetupContext, Signal};

use super::BoxModifier;
use super::types::Alignment;
use super::with_appended_box_modifier;

/// Aligns a child within its available space.
pub struct Align<A: Signal<Value = Alignment>, C: Component> {
    pub alignment: A,
    pub child: C,
}

impl<A, C> Component for Align<A, C>
where
    A: Signal<Value = Alignment> + 'static,
    C: Component + 'static,
{
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let Align { alignment, child } = *self;

        with_appended_box_modifier(ctx, BoxModifier::Align(alignment.read()));
        ctx.boxed_child(Box::new(child));
    }
}
