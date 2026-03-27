use reactive_core::{Component, SetupContext, Signal};

use super::types::Alignment;
use super::with_updated_hints;

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

        with_updated_hints(ctx, move |h| h.alignment = Some(alignment.read()));
        ctx.child(Box::new(child));
    }
}
