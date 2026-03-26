use reactive_core::{Component, ConstantSignal, SetupContext, Signal};
use std::num::NonZeroUsize;

use super::with_updated_hints;

/// Expands a child to fill its share of remaining space in a Row/Column.
/// Equivalent to Flutter's `Expanded`.
pub struct Expanded<F: Signal<Value = Option<NonZeroUsize>>, C: Component> {
    pub flex: F,
    pub child: C,
}

impl<F, C> Component for Expanded<F, C>
where
    F: Signal<Value = Option<NonZeroUsize>> + 'static,
    C: Component,
{
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let Expanded { flex, child } = *self;
        with_updated_hints(ctx, move |h| h.flex = flex.read());
        ctx.child(child);
    }
}
