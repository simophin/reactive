use jni::{objects::JObject, JNIEnv};

use crate::ToJavaValue;

impl<T: ToJavaValue> ToJavaValue for Option<&T> {
    type JavaType<'a> = JObject<'a>;
    type ConvertError = T::BoxingError;
    type BoxingError = T::BoxingError;

    fn into_java_value<'s>(
        &self,
        env: &mut JNIEnv<'s>,
    ) -> Result<Self::JavaType<'s>, Self::ConvertError> {
        match self {
            Some(v) => (*v).into_java_value_boxed(env),
            None => Ok(JObject::null()),
        }
    }

    fn into_java_value_boxed<'s>(
        &self,
        env: &mut JNIEnv<'s>,
    ) -> Result<JObject<'s>, Self::BoxingError> {
        self.into_java_value(env)
    }
}
