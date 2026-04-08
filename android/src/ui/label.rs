use ui_core::Prop;
use crate::ui::view_component::{AndroidView, AndroidViewBuilder, AndroidViewComponent};
use reactive_core::{Component, SetupContext, Signal};
use ui_core::widgets::Label;

pub struct AndroidLabel;

impl Label for AndroidLabel {
    fn new(text: impl Signal<Value = String>) -> Self {
        AndroidViewComponent(
            AndroidViewBuilder::create_no_child(
                |_ctx| {
                    // In real impl, this calls JNI to create a TextView
                    todo!("Create TextView via JNI")
                },
                |w| todo!("Convert to AndroidView"),
            )
            .bind(PROP_TEXT, text)
        )
    }

    fn font_size(self, size: impl Signal<Value = f64>) -> Self {
        // bind PROP_FONT_SIZE
        self
    }

    fn alignment(self, alignment: ui_core::layout::TextAlignment) -> Self {
        // bind PROP_ALIGNMENT
        self
    }
}

pub static PROP_TEXT: &Prop<AndroidLabel, AndroidView, String> =
    &Prop::new(|view, text| {
        let mut env = view.env();
        let j_text = env.new_string(&text).unwrap();
        env.call_method(view.as_obj(), "setText", "(Ljava/lang/CharSequence;)V", &[&j_text]).unwrap();
    });
