use super::ImageView;
use crate::Prop;
use crate::android::JavaObject;
use crate::android::java;
use crate::widgets::{self, ImageCodec, NativeView};
use jni::objects::JValue;
use reactive_core::{Signal, SignalExt};
use std::error::Error;

pub type Image = NativeView<JavaObject, ImageView>;

/// A decoded `android.graphics.Bitmap`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bitmap(pub JavaObject);

pub static PROP_IMAGE: Prop<Image, ImageView, Bitmap> = Prop::new(|view, bitmap| {
    view.call_void(
        "setImageBitmap",
        "(Landroid/graphics/Bitmap;)V",
        &[JValue::Object(bitmap.0.as_obj())],
    )
});

pub static PROP_CONTENT_DESCRIPTION: Prop<Image, ImageView, Option<String>> =
    Prop::new(|view, desc| match desc {
        Some(desc) => view.set_text("setContentDescription", &desc),
        None => view.call_void(
            "setContentDescription",
            "(Ljava/lang/CharSequence;)V",
            &[JValue::Object(&jni::objects::JObject::null())],
        ),
    });

impl widgets::Image for Image {
    type NativeHandle = Bitmap;

    fn new<S: Into<String>>(
        image: impl Signal<Value = Bitmap> + 'static,
        desc: Option<impl Signal<Value = S> + 'static>,
    ) -> Self {
        NativeView::new(
            |ctx| {
                let view = ImageView(super::new_view(ctx, "android/widget/ImageView"));
                // Keep the measured size in the bitmap's aspect ratio when
                // only one dimension is constrained.
                view.call_void("setAdjustViewBounds", "(Z)V", &[JValue::Bool(1)]);
                view
            },
            Into::into,
            |_, _| {},
            Default::default(),
            &super::VIEW_REGISTRY_KEY,
        )
        .bind(PROP_IMAGE, image)
        .bind(
            PROP_CONTENT_DESCRIPTION,
            desc.map_value(|d| d.map(Into::into)),
        )
    }
}

/// Decodes images with `BitmapFactory`. Safe to call from any thread.
pub struct BitmapCodec;

impl BitmapCodec {
    fn decode(data: &[u8]) -> Result<Bitmap, Box<dyn Error + Send + Sync>> {
        let mut env = java::vm().attach_current_thread()?;
        env.with_local_frame(4, |env| {
            let bytes = env.byte_array_from_slice(data)?;
            let bitmap = env
                .call_static_method(
                    "android/graphics/BitmapFactory",
                    "decodeByteArray",
                    "([BII)Landroid/graphics/Bitmap;",
                    &[
                        JValue::Object(&bytes),
                        JValue::Int(0),
                        JValue::Int(data.len() as i32),
                    ],
                )
                .and_then(|v| v.l());

            match bitmap {
                Ok(bitmap) if !bitmap.is_null() => Ok(Bitmap(JavaObject::new(env, &bitmap))),
                Ok(_) => Err("BitmapFactory could not decode the image".into()),
                Err(err) => {
                    let _ = env.exception_clear();
                    Err(err.into())
                }
            }
        })
    }
}

impl ImageCodec for BitmapCodec {
    type NativeHandle = Bitmap;

    fn decode_static(data: &'static [u8]) -> Result<Bitmap, Box<dyn Error + Send + Sync>> {
        Self::decode(data)
    }

    fn decode_owned(data: Vec<u8>) -> Result<Bitmap, Box<dyn Error + Send + Sync>> {
        Self::decode(&data)
    }
}
