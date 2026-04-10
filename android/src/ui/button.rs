use std::cell::RefCell;
use std::rc::Rc;

use jni::objects::{JObject, JValue};
use reactive_core::Signal;
use ui_core::widgets::Button;
use ui_core::Prop;

use crate::bindings;
use crate::ui::click_listener::ReactiveOnClickListener;
use crate::ui::view_component::{AndroidView, AndroidViewBuilder, AndroidViewComponent};

pub type AndroidButton = AndroidViewComponent<AndroidView, ui_core::NoChild>;

pub static PROP_TEXT: &Prop<AndroidButton, AndroidView, String> = &Prop::new(|view, text| {
    let mut env = view.env();
    let java_text = bindings::new_java_string(&mut env, &text).expect("create button text");
    let java_text_obj = JObject::from(java_text);
    bindings::call_void::<bindings::text_view::setText, (jni::sys::jobject,)>(
        &mut env,
        view.as_obj(),
        &[JValue::Object(&java_text_obj)],
    )
    .expect("set button text");
});

pub static PROP_ENABLED: &Prop<AndroidButton, AndroidView, bool> = &Prop::new(|view, enabled| {
    let mut env = view.env();
    bindings::call_void::<bindings::view::setEnabled, (jni::sys::jboolean,)>(
        &mut env,
        view.as_obj(),
        &[JValue::Bool(enabled as u8)],
    )
    .expect("set button enabled");
});

impl Button for AndroidButton {
    fn new(title: impl Signal<Value = String> + 'static, on_click: impl Fn() + 'static) -> Self {
        let on_click = Rc::new(on_click);
        AndroidViewComponent(
            AndroidViewBuilder::create_no_child(
                move |_ctx| {
                    let java_vm = AndroidView::java_vm();
                    let mut env = java_vm
                        .attach_current_thread_permanently()
                        .expect("attach thread");
                    let activity = AndroidView::activity();
                    let button_obj = bindings::new_object::<bindings::button::Button>(
                        &mut env,
                        "(Landroid/content/Context;)V",
                        &[JValue::Object(activity.as_obj())],
                    )
                    .expect("create Button");

                    let listener = ReactiveOnClickListener {
                        on_click: RefCell::new(Some(on_click.clone())),
                    };
                    let activity_obj = env
                        .new_local_ref(activity.as_obj())
                        .expect("clone activity ref");
                    let listener_obj = listener
                        .into_java(&mut env, activity_obj)
                        .expect("create click listener");

                    bindings::call_void::<bindings::view::setOnClickListener, (jni::sys::jobject,)>(
                        &mut env,
                        &button_obj,
                        &[JValue::Object((&listener_obj).into())],
                    )
                    .expect("attach click listener");

                    AndroidView::new(&mut env, &button_obj)
                },
                |v| v,
            )
            .bind(PROP_TEXT, title),
        )
    }

    fn enabled(self, value: impl Signal<Value = bool> + 'static) -> Self {
        AndroidViewComponent(self.0.bind(PROP_ENABLED, value))
    }
}
