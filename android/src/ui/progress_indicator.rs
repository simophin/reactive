use jni::objects::JValue;
use reactive_core::{IntoSignal, Signal, SignalExt};
use ui_core::widgets::ProgressIndicator;
use ui_core::Prop;

use crate::bindings;
use crate::ui::view_component::{AndroidView, AndroidViewBuilder, AndroidViewComponent};

pub type AndroidProgressIndicator = AndroidViewComponent<AndroidView, ui_core::NoChild>;

pub static PROP_PROGRESS: &Prop<AndroidProgressIndicator, AndroidView, i32> =
    &Prop::new(|view, value| {
        let mut env = view.env();
        bindings::call_void::<bindings::progress_bar::setProgress, (jni::sys::jint,)>(
            &mut env,
            view.as_obj(),
            &[JValue::Int(value)],
        )
        .expect("set progress");
    });

pub static PROP_MAX: &Prop<AndroidProgressIndicator, AndroidView, i32> = &Prop::new(|view, max| {
    let mut env = view.env();
    bindings::call_void::<bindings::progress_bar::setMax, (jni::sys::jint,)>(
        &mut env,
        view.as_obj(),
        &[JValue::Int(max)],
    )
    .expect("set progress max");
});

fn create_progress_bar(indeterminate: bool) -> AndroidView {
    let java_vm = AndroidView::java_vm();
    let mut env = java_vm
        .attach_current_thread_permanently()
        .expect("attach thread");
    let activity = AndroidView::activity();
    let progress_bar = bindings::new_object::<bindings::progress_bar::ProgressBar>(
        &mut env,
        "(Landroid/content/Context;)V",
        &[JValue::Object(activity.as_obj())],
    )
    .expect("create ProgressBar");

    bindings::call_void::<bindings::progress_bar::setIndeterminate, (jni::sys::jboolean,)>(
        &mut env,
        &progress_bar,
        &[JValue::Bool(indeterminate as u8)],
    )
    .expect("set progress mode");

    AndroidView::new(&mut env, &progress_bar)
}

impl ProgressIndicator for AndroidProgressIndicator {
    fn new_bar(value: impl Signal<Value = usize> + 'static) -> Self {
        AndroidViewComponent(
            AndroidViewBuilder::create_no_child(|_| create_progress_bar(false), |v| v)
                .bind(PROP_MAX, 100_i32.into_signal())
                .bind(PROP_PROGRESS, value.map_value(|v| v as i32)),
        )
    }

    fn new_spinner() -> Self {
        AndroidViewComponent(AndroidViewBuilder::create_no_child(
            |_| create_progress_bar(true),
            |v| v,
        ))
    }
}
