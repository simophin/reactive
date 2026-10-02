use super::leaf;
use crate::Prop;
use crate::android::node::check;
use crate::android::{JniArg, Node, env};
use crate::widgets::{self, ImageCodec, NativeView};
use jni::JNIEnv;
use jni::objects::{GlobalRef, JValue};
use reactive_core::{Signal, SignalExt};
use std::error::Error;

pub type ImageView = NativeView<Node, Node>;

/// A decoded `android.graphics.Bitmap`.
#[derive(Clone)]
pub struct ImageHandle(GlobalRef);

impl PartialEq for ImageHandle {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_obj().as_raw() == other.0.as_obj().as_raw()
    }
}

impl Eq for ImageHandle {}

impl JniArg for &ImageHandle {
    const SIG: &'static str = "Landroid/graphics/Bitmap;";

    fn with_jvalue<R>(&self, env: &mut JNIEnv, f: impl FnOnce(&mut JNIEnv, JValue) -> R) -> R {
        f(env, JValue::Object(self.0.as_obj()))
    }
}

pub struct AndroidImageCodec;

impl ImageCodec for AndroidImageCodec {
    type NativeHandle = ImageHandle;

    fn decode_static(data: &'static [u8]) -> Result<ImageHandle, Box<dyn Error + Send + Sync>> {
        decode(data)
    }

    fn decode_owned(data: Vec<u8>) -> Result<ImageHandle, Box<dyn Error + Send + Sync>> {
        decode(&data)
    }
}

fn decode(data: &[u8]) -> Result<ImageHandle, Box<dyn Error + Send + Sync>> {
    let mut env = env();
    let bytes = env.byte_array_from_slice(data)?;
    let bitmap = env.call_static_method(
        "com/reactive/ImageNode",
        "decode",
        "([B)Landroid/graphics/Bitmap;",
        &[JValue::Object(&bytes)],
    );
    let bitmap = check(&mut env, bitmap, "ImageNode.decode").l()?;
    if bitmap.is_null() {
        return Err("BitmapFactory could not decode the image".into());
    }
    Ok(ImageHandle(env.new_global_ref(bitmap)?))
}

pub static PROP_IMAGE: Prop<ImageView, Node, ImageHandle> =
    Prop::new(|n, v| n.set("setBitmap", &v));
pub static PROP_DESCRIPTION: Prop<ImageView, Node, Option<String>> =
    Prop::new(|n, v| n.set("setDescription", v.as_deref()));

impl widgets::Image for ImageView {
    type NativeHandle = ImageHandle;

    fn new<S: Into<String>>(
        image: impl Signal<Value = ImageHandle> + 'static,
        desc: Option<impl Signal<Value = S> + 'static>,
    ) -> Self {
        leaf(|_| Node::new("com/reactive/ImageNode"))
            .bind(PROP_IMAGE, image)
            .bind(PROP_DESCRIPTION, desc.map_value(|d| d.map(Into::into)))
    }
}
