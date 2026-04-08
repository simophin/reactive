use ui_core::widgets::TextInput;
use crate::ui::view_component::{AndroidViewBuilder, AndroidViewComponent};
use reactive_core::{Component, SetupContext, Signal};

pub struct AndroidTextInput;

impl TextInput for AndroidTextInput {
    type PlatformTextType = String;
    fn new(value: impl Signal<Value = TextInputState<Self::PlatformTextType>>, on_change: impl FnMut(TextChange<Self::PlatformTextType>)) -> Self {
        AndroidViewComponent(
            AndroidViewBuilder::create_no_child(
                |_ctx| {
                    todo!("Create EditText via JNI")
                },
                |w| todo!("Convert to AndroidView"),
            )
        )
    }

    fn font_size(self, size: impl Signal<Value = f64>) -> Self {
        self
    }
}
