use jni::objects::{JObject, JValue};
use reactive_core::{Signal, SignalExt};
use ui_core::layout::types::TextAlignment;
use ui_core::widgets::Label;
use ui_core::Prop;

use crate::bindings;
use crate::ui::view_component::{AndroidView, AndroidViewBuilder, AndroidViewComponent};

pub type AndroidLabel = AndroidViewComponent<AndroidView, ui_core::NoChild>;

pub static PROP_TEXT: &Prop<AndroidLabel, AndroidView, String> = &Prop::new(|view, text| {
    let mut env = view.env();
    let java_text = bindings::new_java_string(&mut env, &text).expect("create label text");
    let java_text_obj = JObject::from(java_text);
    bindings::call_void::<bindings::text_view::setText, (jni::sys::jobject,)>(
        &mut env,
        view.as_obj(),
        &[JValue::Object(&java_text_obj)],
    )
    .expect("set label text");
});

pub static PROP_FONT_SIZE: &Prop<AndroidLabel, AndroidView, f64> = &Prop::new(|view, size| {
    let mut env = view.env();
    bindings::call_void::<bindings::text_view::setTextSize, (jni::sys::jfloat,)>(
        &mut env,
        view.as_obj(),
        &[JValue::Float(size as f32)],
    )
    .expect("set label size");
});

pub static PROP_ALIGNMENT: &Prop<AndroidLabel, AndroidView, i32> = &Prop::new(|view, alignment| {
    let mut env = view.env();
    bindings::call_void::<bindings::text_view::setTextAlignment, (jni::sys::jint,)>(
        &mut env,
        view.as_obj(),
        &[JValue::Int(alignment)],
    )
    .expect("set label alignment");
});

fn text_alignment_to_android(alignment: TextAlignment) -> i32 {
    match alignment {
        TextAlignment::Leading => 5,
        TextAlignment::Center => 4,
        TextAlignment::Trailing => 6,
    }
}

impl Label for AndroidLabel {
    fn new(text: impl Signal<Value = String> + 'static) -> Self {
        AndroidViewComponent(
            AndroidViewBuilder::create_no_child(
                |_ctx| {
                    let java_vm = AndroidView::java_vm();
                    let mut env = java_vm
                        .attach_current_thread_permanently()
                        .expect("attach thread");
                    let activity = AndroidView::activity();
                    let text_view = bindings::new_object::<bindings::text_view::TextView>(
                        &mut env,
                        "(Landroid/content/Context;)V",
                        &[JValue::Object(activity.as_obj())],
                    )
                    .expect("create TextView");
                    AndroidView::new(&mut env, &text_view)
                },
                |v| v,
            )
            .bind(PROP_TEXT, text),
        )
    }

    fn font_size(self, size: impl Signal<Value = f64> + 'static) -> Self {
        AndroidViewComponent(self.0.bind(PROP_FONT_SIZE, size))
    }

    fn alignment(self, alignment: impl Signal<Value = TextAlignment> + 'static) -> Self {
        AndroidViewComponent(self.0.bind(
            PROP_ALIGNMENT,
            alignment.map_value(text_alignment_to_android),
        ))
    }
}
