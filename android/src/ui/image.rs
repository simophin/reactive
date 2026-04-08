use ui_core::widgets::Image;
use crate::ui::view_component::{AndroidViewBuilder, AndroidViewComponent};
use reactive_core::{Component, SetupContext, Signal};

pub struct AndroidImage;

impl Image for AndroidImage {
    type NativeHandle = GlobalRef; // Placeholder
    fn new(image: impl Signal<Value = Self::NativeHandle>, desc: Option<impl Signal<Value = String>>) -> Self {
        AndroidViewComponent(
            AndroidViewBuilder::create_no_child(
                |_ctx| {
                    todo!("Create ImageView via JNI")
                },
                |w| todo!("Convert to AndroidView"),
            )
        )
    }
}
