use ui_core::widgets::Stack;
use crate::ui::view_component::{AndroidViewBuilder, AndroidViewComponent};
use reactive_core::{BoxedComponent, Component, SetupContext, Signal};
use ui_core::layout::Alignment;

pub struct AndroidStack;

impl Stack for AndroidStack {
    fn new() -> Self {
        AndroidViewComponent(
            AndroidViewBuilder::create_multiple_child(
                |_ctx| {
                    todo!("Create FrameLayout via JNI")
                },
                |w| todo!("Convert to AndroidView"),
            )
        )
    }

    fn alignment(self, alignment: impl Signal<Value = Alignment> + 'static) -> Self {
        // Bind to FrameLayout gravity
        self
    }

    fn child(self, child: impl Component + 'static) -> Self {
        // Add to builder
        self
    }
}
