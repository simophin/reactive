mod sig;
mod value;

use std::fmt::Display;

use jni::{objects::JObject, JNIEnv};

pub trait WithJavaObject {
    fn get_java_object(&self) -> Result<JObject<'_>, jni::errors::Error>;
}

pub trait ToJavaValue {
    type JavaType<'a>;
    type ConvertError: Display;
    type BoxingError: Display;

    fn into_java_value<'s>(
        &self,
        env: &mut JNIEnv<'s>,
    ) -> Result<Self::JavaType<'s>, Self::ConvertError>;

    fn into_java_value_boxed<'s>(
        &self,
        env: &mut JNIEnv<'s>,
    ) -> Result<JObject<'s>, Self::BoxingError>;
}
