use reactive_core::{Component, SetupContext, Signal};

use super::{EdgeInsets, with_updated_hints};

/// Insets a child by the given edge amounts.
pub struct Padding<I: Signal<Value = EdgeInsets>, C> {
    pub insets: I,
    pub child: C,
}

impl<I: Signal<Value = EdgeInsets> + 'static, C: Component + 'static> Component for Padding<I, C> {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let Padding { insets, child } = *self;
        with_updated_hints(ctx, move |h| h.padding = insets.read());
        ctx.child(Box::new(child));
    }
}
