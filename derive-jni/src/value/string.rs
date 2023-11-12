use std::{borrow::Cow, str::Utf8Error};

use jni::{
    objects::{JObject, JString},
    JNIEnv,
};

use crate::{ToJavaValue, ToRustType};

macro_rules! impl_string_like {
    ($item:ty) => {
        impl ToJavaValue for $item {
            type JavaType<'a> = JString<'a>;
            type ConvertError = jni::errors::Error;
            type BoxingError = jni::errors::Error;

            fn into_java_value<'s>(
                &self,
                env: &mut JNIEnv<'s>,
            ) -> Result<Self::JavaType<'s>, Self::ConvertError> {
                env.new_string(self)
            }

            fn into_java_value_boxed<'s>(
                &self,
                env: &mut JNIEnv<'s>,
            ) -> Result<JObject<'s>, Self::BoxingError> {
                self.into_java_value(env).map(|s| s.into())
            }

            fn java_signature() -> Cow<'static, str> {
                Cow::Borrowed("Ljava/lang/String;")
            }

            fn boxed_java_signature() -> Cow<'static, str> {
                Self::java_signature()
            }
        }
    };
}

impl_string_like!(str);
impl_string_like!(String);
impl_string_like!(Cow<'_, str>);

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Error converting to Java type: {0}")]
    Utf8(#[from] Utf8Error),
    #[error("Error with JNI call: {0}")]
    Java(#[from] jni::errors::Error),
}

impl ToRustType<Option<String>> for JString<'_> {
    type BoxedRustType = Option<String>;
    type Error = Error;
    type UnboxingError = Error;

    fn to_rust_type(&self, env: &mut JNIEnv<'_>) -> Result<Option<String>, Self::Error> {
        if self.is_null() {
            return Ok(None);
        }

        let s = env.get_string(&self)?;
        Ok(Some(s.to_str()?.to_owned()))
    }

    fn boxed_to_rust_type<'b>(
        env: &mut JNIEnv<'b>,
        obj: JObject<'b>,
    ) -> Result<Option<String>, Self::UnboxingError> {
        if obj.is_null() {
            return Ok(None);
        }

        let obj = JString::from(obj);
        obj.to_rust_type(env)
    }
}
