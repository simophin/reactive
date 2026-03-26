use reactive_core::{BoxedComponent, Component, SetupContext};

use super::types::Alignment;
use super::with_updated_hints;

/// Centers a child within its available space.
pub struct Center<C: Component> {
    pub child: C,
}

impl<C: Component> Component for Center<C> {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        with_updated_hints(ctx, |h| h.alignment = Some(Alignment::Center));
        ctx.child(self.child);
    }
}
