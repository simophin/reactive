use reactive_core::{Component, SetupContext};
use ui_core::widgets::{Modifier, NativeView, WithModifier};

pub struct GtkViewComponent<W: 'static>(pub(crate) NativeView<gtk4::Widget, W>);

impl<W: Clone + 'static> Component for GtkViewComponent<W> {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        self.0.setup_in_component(ctx);
    }
}

impl<W: Clone + 'static> WithModifier for GtkViewComponent<W> {
    fn modifier(self, modifier: Modifier) -> Self {
        Self(self.0.modifier(modifier))
    }
}
