use std::fmt::Display;

use jni::{
    objects::JObject,
    sys::{jboolean, jbyte, jdouble, jfloat, jint, jlong},
    JNIEnv,
};

use crate::ToJavaValue;

macro_rules! impl_primitive {
    ($t:ty, $j:ty, $jo:literal, $sig:literal) => {
        impl ToJavaValue for $t {
            type JavaType<'a> = $j;
            type ConvertError = <$t as TryInto<$j>>::Error;
            type BoxingError = PrimitiveBoxingError<Self::ConvertError>;

            fn into_java_value<'s>(
                &self,
                _env: &mut JNIEnv<'s>,
            ) -> Result<Self::JavaType<'s>, Self::ConvertError> {
                (*self).try_into()
            }

            fn into_java_value_boxed<'s>(
                &self,
                env: &mut JNIEnv<'s>,
            ) -> Result<JObject<'s>, Self::BoxingError> {
                let primitive_value = self
                    .into_java_value(env)
                    .map_err(|e| PrimitiveBoxingError::ConvertError(e))?;

                Ok(env.new_object($jo, $sig, &[primitive_value.into()])?)
            }
        }
    };
}

#[derive(thiserror::Error)]
pub enum PrimitiveBoxingError<E: Display> {
    #[error("Error converting to Java type: {0}")]
    ConvertError(E),
    #[error("Error with JNI call: {0}")]
    JavaError(#[from] jni::errors::Error),
}

impl_primitive!(bool, jboolean, "java/lang/Boolean", "(Z)V");
impl_primitive!(u8, jbyte, "java/lang/Byte", "(B)V");
impl_primitive!(i8, jbyte, "java/lang/Byte", "(B)V");
impl_primitive!(i32, jint, "java/lang/Integer", "(I)V");
impl_primitive!(u32, jint, "java/lang/Integer", "(I)V");
impl_primitive!(i64, jlong, "java/lang/Long", "(J)V");
impl_primitive!(u64, jlong, "java/lang/Long", "(J)V");
impl_primitive!(f32, jfloat, "java/lang/Float", "(F)V");
impl_primitive!(f64, jdouble, "java/lang/Double", "(D)V");
impl_primitive!(usize, jlong, "java/lang/Long", "(J)V");
impl_primitive!(isize, jlong, "java/lang/Long", "(J)V");
