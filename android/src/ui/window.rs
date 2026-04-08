use ui_core::widgets::Window;
use crate::ui::view_component::{AndroidViewBuilder, AndroidViewComponent};
use reactive_core::{BoxedComponent, Component, SetupContext};

pub struct AndroidWindow;

impl Window for AndroidWindow {
    fn new(title: impl reactive_core::Signal<Value = String>, child: impl Component, width: f64, height: f64) -> Self {
        AndroidViewComponent(
            AndroidViewBuilder::create_with_child(
                |_ctx| {
                    todo!("Create FrameLayout root via JNI")
                },
                |w| todo!("Convert to AndroidView"),
                Box::new(child),
            )
        )
    }
}
