use reactive_core::{Component, SetupContext, Signal, SignalExt};
use std::num::NonZeroUsize;

use super::{FLEX_PARENT_DATA, FlexParentData};

/// Expands a child to fill its share of remaining space in a Row/Column.
/// Equivalent to Flutter's `Expanded`.
pub struct Expanded<F: Signal<Value = Option<NonZeroUsize>>, C: Component> {
    pub flex: F,
    pub child: C,
}

impl<F, C> Component for Expanded<F, C>
where
    F: Signal<Value = Option<NonZeroUsize>> + 'static,
    C: Component + 'static,
{
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let Expanded { flex, child } = *self;
        ctx.set_context(
            &FLEX_PARENT_DATA,
            flex.map_value(|flex| FlexParentData { flex }),
        );

        ctx.child(child);
    }
}
