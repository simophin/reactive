use crate::android::Node;
use crate::widgets::CommonWindow;
use reactive_core::{Component, SetupContext};

/// The activity is the window on Android: the child renders into the host's `RootNode`.
/// Title and size are ignored.
pub type Window = CommonWindow<Node>;

impl Component for Window {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        ctx.boxed_child(self.child);
    }
}
