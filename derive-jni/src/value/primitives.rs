use jni::{
    objects::JObject,
    sys::{jboolean, jbyte, jdouble, jfloat, jint, jlong},
    JNIEnv,
};

use crate::{ToJavaValue, ToRustType};
use std::borrow::Cow;
use std::convert::Infallible;

macro_rules! impl_primitive_to_java {
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

                Ok(env.new_object($jo, concat!("(", $sig, ")V"), &[primitive_value.into()])?)
            }

            fn java_signature() -> Cow<'static, str> {
                Cow::Borrowed($sig)
            }

            fn boxed_java_signature() -> Cow<'static, str> {
                let sig = concat!("L", $jo, ";");
                sig.replace(".", "/").into()
            }
        }
    };
}

#[derive(thiserror::Error, Debug)]
pub enum PrimitiveBoxingError<E: std::error::Error> {
    #[error("Error converting to Java type: {0}")]
    ConvertError(E),
    #[error("Error with JNI call: {0}")]
    JavaError(#[from] jni::errors::Error),
}

impl_primitive_to_java!(bool, jboolean, "java/lang/Boolean", "Z");
impl_primitive_to_java!(u8, jbyte, "java/lang/Byte", "B");
impl_primitive_to_java!(i8, jbyte, "java/lang/Byte", "B");
impl_primitive_to_java!(i32, jint, "java/lang/Integer", "I");
impl_primitive_to_java!(u32, jint, "java/lang/Integer", "I");
impl_primitive_to_java!(i64, jlong, "java/lang/Long", "J");
impl_primitive_to_java!(u64, jlong, "java/lang/Long", "J");
impl_primitive_to_java!(f32, jfloat, "java/lang/Float", "F");
impl_primitive_to_java!(f64, jdouble, "java/lang/Double", "D");
impl_primitive_to_java!(usize, jlong, "java/lang/Long", "J");
impl_primitive_to_java!(isize, jlong, "java/lang/Long", "J");

impl ToJavaValue for () {
    type JavaType<'a> = ();
    type ConvertError = Infallible;
    type BoxingError = Infallible;

    fn into_java_value<'s>(
        &self,
        _env: &mut JNIEnv<'s>,
    ) -> Result<Self::JavaType<'s>, Self::ConvertError> {
        Ok(())
    }

    fn into_java_value_boxed<'s>(
        &self,
        _env: &mut JNIEnv<'s>,
    ) -> Result<JObject<'s>, Self::BoxingError> {
        Ok(JObject::null())
    }

    fn java_signature() -> Cow<'static, str> {
        Cow::Borrowed("V")
    }

    fn boxed_java_signature() -> Cow<'static, str> {
        Cow::Borrowed("V")
    }
}

impl ToRustType<bool> for jboolean {
    type Error = Infallible;
    type BoxedRustType = Option<bool>;
    type UnboxingError = jni::errors::Error;

    fn to_rust_type(&self, _env: &mut JNIEnv<'_>) -> Result<bool, Self::Error> {
        Ok(*self != 0)
    }

    fn boxed_to_rust_type<'a>(
        env: &mut JNIEnv<'a>,
        obj: JObject<'a>,
    ) -> Result<Self::BoxedRustType, Self::UnboxingError> {
        if env.is_same_object(&obj, JObject::null())? {
            return Ok(None);
        }

        let result = env.call_method(&obj, "booleanValue", "()Z", &[])?;
        Ok(Some(result.z()?))
    }
}

macro_rules! impl_primitive_to_rust {
    ($t:ty, $j:ty, $result_method:ident, $jmethod:literal, $sig:literal) => {
        impl ToRustType<$t> for $j {
            type Error = Infallible;
            type BoxedRustType = Option<$t>;
            type UnboxingError = jni::errors::Error;

            fn to_rust_type(&self, _env: &mut JNIEnv<'_>) -> Result<$t, Self::Error> {
                Ok(*self as $t)
            }

            fn boxed_to_rust_type<'a>(
                env: &mut JNIEnv<'a>,
                obj: JObject<'a>,
            ) -> Result<Self::BoxedRustType, Self::UnboxingError> {
                if env.is_same_object(&obj, JObject::null())? {
                    return Ok(None);
                }

                let result = env.call_method(&obj, $jmethod, $sig, &[])?;
                Ok(Some(result.$result_method()? as $t))
            }
        }
    };
}

impl_primitive_to_rust!(u8, jbyte, b, "byteValue", "()B");
impl_primitive_to_rust!(i8, jbyte, b, "byteValue", "()B");
impl_primitive_to_rust!(i32, jint, i, "intValue", "()I");
impl_primitive_to_rust!(u32, jint, i, "intValue", "()I");
impl_primitive_to_rust!(i64, jlong, j, "longValue", "()J");
impl_primitive_to_rust!(u64, jlong, j, "longValue", "()J");
impl_primitive_to_rust!(f32, jfloat, f, "floatValue", "()F");
impl_primitive_to_rust!(f64, jdouble, d, "doubleValue", "()D");
impl_primitive_to_rust!(usize, jlong, j, "longValue", "()J");
impl_primitive_to_rust!(isize, jlong, j, "longValue", "()J");
