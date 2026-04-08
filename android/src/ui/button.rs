use ui_core::Prop;
use crate::ui::view_component::{AndroidView, AndroidViewBuilder, AndroidViewComponent};
use reactive_core::{Component, SetupContext, Signal};
use ui_core::widgets::Button;

pub struct AndroidButton;

impl Button for AndroidButton {
    fn new(title: impl Signal<Value = String>, on_click: impl Fn()) -> Self {
        AndroidViewComponent(
            AndroidViewBuilder::create_no_child(
                |_ctx| {
                    todo!("Create Button via JNI")
                },
                |w| todo!("Convert to AndroidView"),
            )
            .bind(PROP_TEXT, title)
        )
    }

    fn enabled(self, value: impl Signal<Value = bool>) -> Self {
        // bind PROP_ENABLED
        self
    }
}

pub static PROP_TEXT: &Prop<AndroidButton, AndroidView, String> =
    &Prop::new(|view, text| {
        let mut env = view.env();
        let j_text = env.new_string(&text).unwrap();
        env.call_method(view.as_obj(), "setText", "(Ljava/lang/CharSequence;)V", &[&j_text]).unwrap();
    });
pub static PROP_ENABLED: &Prop<AndroidButton, AndroidView, bool> =
    &Prop::new(|view, enabled| {
        let mut env = view.env();
        env.call_method(view.as_obj(), "setEnabled", "(Z)V", &[jni::objects::JValue::Bool(enabled)]).unwrap();
    });
