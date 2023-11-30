use jni::{
    objects::JObject,
    sys::{jboolean, jbyte, jchar, jdouble, jfloat, jint, jlong, jshort, jvalue},
};

pub trait IntoJValue {
    fn into_jvalue(self) -> jvalue;
}

macro_rules! impl_java_primitive {
    ($t:ty, $value_ident:ident) => {
        impl IntoJValue for $t {
            fn into_jvalue(self) -> jvalue {
                jvalue { $value_ident: self }
            }
        }
    };
}

impl_java_primitive!(jboolean, z);
impl_java_primitive!(jbyte, b);
impl_java_primitive!(jint, i);
impl_java_primitive!(jlong, j);
impl_java_primitive!(jshort, s);
impl_java_primitive!(jfloat, f);
impl_java_primitive!(jdouble, d);
impl_java_primitive!(jchar, c);

impl IntoJValue for &'_ JObject<'_> {
    fn into_jvalue(self) -> jvalue {
        jvalue { l: self.as_raw() }
    }
}
