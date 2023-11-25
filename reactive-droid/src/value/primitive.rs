use jni::{
    errors::Result,
    objects::{AutoLocal, JObject, JValueGen},
    sys::{jboolean, jbyte, jchar, jdouble, jfloat, jint, jlong, jshort},
    JNIEnv,
};

use super::IntoJValue;

macro_rules! impl_primitive {
    ($ty:ty, $variant:ident, $cast:ty) => {
        impl IntoJValue for $ty {
            fn into_jvalue<'local>(
                &self,
                _env: &mut JNIEnv<'local>,
            ) -> Result<JValueGen<AutoLocal<'local, JObject<'local>>>> {
                Ok(JValueGen::$variant(*self as $cast))
            }
        }
    };
}

impl_primitive!(u8, Byte, jbyte);
impl_primitive!(i8, Byte, jbyte);
impl_primitive!(u16, Short, jshort);
impl_primitive!(i16, Short, jshort);
impl_primitive!(u32, Int, jint);
impl_primitive!(i32, Int, jint);
impl_primitive!(u64, Long, jlong);
impl_primitive!(i64, Long, jlong);
impl_primitive!(bool, Bool, jboolean);
impl_primitive!(char, Char, jchar);
impl_primitive!(f32, Float, jfloat);
impl_primitive!(f64, Double, jdouble);
