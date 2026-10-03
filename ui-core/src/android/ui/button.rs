use super::ButtonView;
use crate::Prop;
use crate::android::JavaObject;
use crate::android::callback::new_callback;
use crate::widgets::{self, NativeView};
use jni::objects::{JObject, JValue};
use reactive_core::Signal;

pub type Button = NativeView<JavaObject, ButtonView>;

pub static PROP_TITLE: Prop<Button, ButtonView, String> =
    Prop::new(|view, title| view.set_text("setText", &title));

pub static PROP_ENABLED: Prop<Button, ButtonView, bool> = Prop::new(|view, enabled| {
    view.call_void("setEnabled", "(Z)V", &[JValue::Bool(enabled.into())])
});

const SET_ON_CLICK: &str = "setOnClickListener";
const SET_ON_CLICK_SIG: &str = "(Landroid/view/View$OnClickListener;)V";

impl widgets::Button for Button {
    fn new(title: impl Signal<Value = String> + 'static, on_click: impl Fn() + 'static) -> Self {
        NativeView::new(
            move |ctx| {
                let button = ButtonView(super::new_view(ctx, "android/widget/Button"));
                let listener = new_callback(on_click);
                button.call_void(
                    SET_ON_CLICK,
                    SET_ON_CLICK_SIG,
                    &[JValue::Object(listener.as_obj())],
                );

                let button_ = button.clone();
                ctx.on_cleanup(move || {
                    button_.call_void(
                        SET_ON_CLICK,
                        SET_ON_CLICK_SIG,
                        &[JValue::Object(&JObject::null())],
                    );
                    listener.call_void("release", "()V", &[]);
                });
                button
            },
            Into::into,
            |_, _| {},
            Default::default(),
            &super::VIEW_REGISTRY_KEY,
        )
        .bind(PROP_TITLE, title)
    }

    fn enabled(self, value: impl Signal<Value = bool> + 'static) -> Self {
        self.bind(PROP_ENABLED, value)
    }
}
