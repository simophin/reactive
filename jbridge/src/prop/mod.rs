use jni::{
    errors::Result as JResult,
    objects::{JObject, JValueGen, JValueOwned},
    JNIEnv,
};

pub trait PropertyAccess {
    fn get_property<'local>(
        &self,
        env: &mut JNIEnv<'local>,
        obj: &JObject<'local>,
        name: &str,
    ) -> JResult<JValueOwned<'local>>;

    fn set_property<'local>(
        &mut self,
        env: &mut JNIEnv<'local>,
        obj: &JObject<'local>,
        name: &str,
        value: &JValueGen<JObject<'local>>,
    ) -> JResult<()>;
}

mod read;
mod write;

pub use read::*;
