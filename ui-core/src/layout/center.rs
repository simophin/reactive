use reactive_core::{Component, SetupContext};

use super::BoxModifier;
use super::types::Alignment;
use super::with_appended_box_modifier;

/// Centers a child within its available space.
pub struct Center<C: Component> {
    pub child: C,
}

impl<C: Component + 'static> Component for Center<C> {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        with_appended_box_modifier(ctx, BoxModifier::Align(Alignment::Center));
        ctx.boxed_child(Box::new(self.child));
    }
}
