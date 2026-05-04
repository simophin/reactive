use jni::objects::{JObject, JValue};
use reactive_core::{Signal, SignalExt};
use ui_core::widgets::Image;
use ui_core::Prop;

use crate::android::bindings;
use crate::android::ui::view_component::{AndroidView, AndroidViewBuilder, AndroidViewComponent};

pub type AndroidImageView = AndroidViewComponent<AndroidView, ui_core::NoChild>;
pub type AndroidImage = AndroidImageView;

pub struct AndroidImageCodec;

impl ui_core::widgets::ImageCodec for AndroidImageCodec {
    type NativeHandle = AndroidView;

    fn decode_static(
        _data: &'static [u8],
    ) -> Result<Self::NativeHandle, Box<dyn std::error::Error + Send + Sync>> {
        Err("Android image decoding is not wired up yet".into())
    }

    fn decode_owned(
        _data: Vec<u8>,
    ) -> Result<Self::NativeHandle, Box<dyn std::error::Error + Send + Sync>> {
        Err("Android image decoding is not wired up yet".into())
    }
}

pub static PROP_IMAGE: &Prop<AndroidImage, AndroidView, AndroidView> =
    &Prop::new(|view, handle| {
        let mut env = view.env();
        bindings::call_void::<bindings::image_view::setImageDrawable, (jni::sys::jobject,)>(
            &mut env,
            view.as_obj(),
            &[JValue::Object(handle.as_obj())],
        )
        .expect("set image drawable");
    });

pub static PROP_CONTENT_DESCRIPTION: &Prop<AndroidImage, AndroidView, String> =
    &Prop::new(|view, desc| {
        let mut env = view.env();
        let java_desc = bindings::new_java_string(&mut env, &desc).expect("content description");
        let java_desc_obj = JObject::from(java_desc);
        bindings::call_void::<bindings::image_view::setContentDescription, (jni::sys::jobject,)>(
            &mut env,
            view.as_obj(),
            &[JValue::Object(&java_desc_obj)],
        )
        .expect("set content description");
    });

impl Image for AndroidImage {
    type NativeHandle = AndroidView;

    fn new<S: Into<String>>(
        image: impl Signal<Value = Self::NativeHandle> + 'static,
        desc: Option<impl Signal<Value = S> + 'static>,
    ) -> Self {
        let mut builder = AndroidViewBuilder::create_no_child(
            |_ctx| {
                let java_vm = AndroidView::java_vm();
                let mut env = java_vm
                    .attach_current_thread_permanently()
                    .expect("attach thread");
                let activity = AndroidView::activity();
                let image_view = bindings::new_object::<bindings::image_view::ImageView>(
                    &mut env,
                    "(Landroid/content/Context;)V",
                    &[JValue::Object(activity.as_obj())],
                )
                .expect("create ImageView");
                AndroidView::new(&mut env, &image_view)
            },
            |v| v,
        )
        .bind(PROP_IMAGE, image);

        if let Some(desc) = desc {
            builder = builder.bind(
                PROP_CONTENT_DESCRIPTION,
                desc.map_value(|value| value.into()),
            );
        }

        AndroidViewComponent(builder)
    }
}
