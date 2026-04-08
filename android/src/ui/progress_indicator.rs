use ui_core::widgets::ProgressIndicator;
use crate::ui::view_component::{AndroidViewBuilder, AndroidViewComponent};
use reactive_core::{Component, SetupContext, Signal};

pub struct AndroidProgressIndicator;

impl ProgressIndicator for AndroidProgressIndicator {
    fn new_bar(value: impl Signal<Value = usize>) -> Self {
        AndroidViewComponent(
            AndroidViewBuilder::create_no_child(
                |_ctx| {
                    todo!("Create ProgressBar (determinate) via JNI")
                },
                |w| todo!("Convert to AndroidView"),
            )
            // bind progress
        )
    }

    fn new_spinner() -> Self {
        AndroidViewComponent(
            AndroidViewBuilder::create_no_child(
                |_ctx| {
                    todo!("Create ProgressBar (indeterminate) via JNI")
                },
                |w| todo!("Convert to AndroidView"),
            )
        )
    }
}
