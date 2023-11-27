pub mod ext;
mod primitive;
mod strings;

use jni::{
    errors::Result,
    objects::{AutoLocal, JObject, JValueGen},
    JNIEnv,
};

pub trait IntoJValue {
    fn into_jvalue<'local>(
        &self,
        env: &mut JNIEnv<'local>,
    ) -> Result<JValueGen<AutoLocal<'local, JObject<'local>>>>;
}
