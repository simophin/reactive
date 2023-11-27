use jni::{
    objects::{JObject, JString, JValueGen},
    sys::JNI_TRUE,
};

use crate::ToRustType;

impl<T> ToRustType<bool> for JValueGen<T> {
    type BoxedRustType = Option<bool>;
    type Error = jni::errors::Error;
    type UnboxingError = jni::errors::Error;

    fn to_rust_type(&self, _env: &mut jni::JNIEnv<'_>) -> Result<bool, Self::Error> {
        match self {
            JValueGen::Bool(b) => Ok(*b == JNI_TRUE),
            v => Err(jni::errors::Error::WrongJValueType("bool", v.type_name())),
        }
    }

    fn boxed_to_rust_type<'a>(
        env: &mut jni::JNIEnv<'a>,
        obj: jni::objects::JObject<'a>,
    ) -> Result<Self::BoxedRustType, Self::UnboxingError> {
        if obj.is_null() {
            return Ok(None);
        }

        Ok(Some(env.call_method(obj, "booleanValue", "()Z", &[])?.z()?))
    }
}

#[derive(thiserror::Error, Debug)]
pub enum StringToRustError {
    #[error("UTF-8 error: {0}")]
    Utf8Error(std::str::Utf8Error),
    #[error("JNI error: {0}")]
    JniError(#[from] jni::errors::Error),
}

impl ToRustType<Option<String>> for JValueGen<JObject<'_>> {
    type Error = StringToRustError;
    type BoxedRustType = Option<String>;
    type UnboxingError = StringToRustError;

    fn to_rust_type(&self, env: &mut jni::JNIEnv<'_>) -> Result<Option<String>, Self::Error> {
        match self {
            JValueGen::Object(s) if s.is_null() => Ok(None),
            JValueGen::Object(s) => Ok(Some(
                env.get_string(&unsafe { JString::from_raw(s.cast()) })?
                    .to_str()
                    .map_err(|e| StringToRustError::Utf8Error(e))?
                    .to_owned(),
            )),
            v => Err(jni::errors::Error::WrongJValueType("String", v.type_name()).into()),
        }
    }

    fn boxed_to_rust_type<'a>(
        env: &mut jni::JNIEnv<'a>,
        obj: jni::objects::JObject<'a>,
    ) -> Result<Self::BoxedRustType, Self::UnboxingError> {
        JValueGen::Object(obj).to_rust_type(env)
    }
}

macro_rules! impl_to_rust {
    ($jvalue:ident, $rt:ty, $method:literal, $sig:literal, $result_method:ident) => {
        impl<T> ToRustType<$rt> for JValueGen<T> {
            type Error = jni::errors::Error;
            type BoxedRustType = Option<$rt>;
            type UnboxingError = jni::errors::Error;

            fn to_rust_type(&self, _env: &mut jni::JNIEnv<'_>) -> Result<$rt, Self::Error> {
                match self {
                    JValueGen::$jvalue(d) => Ok(*d as $rt),
                    v => Err(jni::errors::Error::WrongJValueType(
                        stringify!($jt),
                        v.type_name(),
                    )),
                }
            }

            fn boxed_to_rust_type<'a>(
                env: &mut jni::JNIEnv<'a>,
                obj: jni::objects::JObject<'a>,
            ) -> Result<Self::BoxedRustType, Self::UnboxingError> {
                if obj.is_null() {
                    return Ok(None);
                }

                Ok(Some(
                    env.call_method(obj, $method, $sig, &[])?.$result_method()? as $rt,
                ))
            }
        }
    };
}

impl_to_rust!(Byte, i8, "byteValue", "()B", b);
impl_to_rust!(Byte, u8, "byteValue", "()B", b);
impl_to_rust!(Int, i32, "intValue", "()I", i);
impl_to_rust!(Int, u32, "intValue", "()I", i);
impl_to_rust!(Short, i16, "shortValue", "()S", s);
impl_to_rust!(Short, u16, "shortValue", "()S", s);
impl_to_rust!(Long, i64, "longValue", "()J", j);
impl_to_rust!(Long, u64, "longValue", "()J", j);
