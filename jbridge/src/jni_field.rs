use jni::{
    objects::{JClass, JFieldID, JObject, JStaticFieldID},
    sys::jboolean,
    JNIEnv,
};

pub trait StaticFieldValue {
    fn apply(&self, env: &mut JNIEnv<'_>, c: &JClass<'_>, id: JStaticFieldID);
    fn get(env: &mut JNIEnv<'_>, c: &JClass<'_>, id: JStaticFieldID) -> Self;
}

pub trait FieldValue {
    fn apply(&self, env: &mut JNIEnv<'_>, c: &JObject<'_>, id: JFieldID);
    fn get(env: &mut JNIEnv<'_>, c: &JObject<'_>, id: JFieldID) -> Self;
}

macro_rules! impl_value {
    ($t:ty, $sm_static:ident, $gm_static:ident, $sm:ident, $gm:ident) => {
        impl StaticFieldValue for $t {
            fn apply(&self, env: &mut JNIEnv<'_>, c: &JClass<'_>, id: JStaticFieldID) {
                let raw_env = env.get_raw();
                unsafe {
                    (**raw_env).$sm_static.unwrap()(raw_env, c.as_raw(), id.into_raw(), *self);
                }
            }

            fn get(env: &mut JNIEnv<'_>, c: &JClass<'_>, id: JStaticFieldID) -> Self {
                let env = env.get_raw();
                unsafe { (**env).$gm_static.unwrap()(env, c.as_raw(), id.into_raw()) }
            }
        }

        impl FieldValue for $t {
            fn apply(&self, env: &mut JNIEnv<'_>, c: &JObject<'_>, id: JFieldID) {
                let env = env.get_raw();
                unsafe {
                    (**env).$sm.unwrap()(env, c.as_raw(), id.into_raw(), *self);
                }
            }

            fn get(env: &mut JNIEnv<'_>, c: &JObject<'_>, id: JFieldID) -> Self {
                let env = env.get_raw();
                unsafe { (**env).$gm.unwrap()(env, c.as_raw(), id.into_raw()) }
            }
        }
    };
}

impl_value!(
    jboolean,
    SetStaticBooleanField,
    GetStaticBooleanField,
    SetBooleanField,
    GetBooleanField
);

impl_value!(
    i8,
    SetStaticByteField,
    GetStaticByteField,
    SetByteField,
    GetByteField
);

impl_value!(
    i16,
    SetStaticShortField,
    GetStaticShortField,
    SetShortField,
    GetShortField
);

impl_value!(
    i32,
    SetStaticIntField,
    GetStaticIntField,
    SetIntField,
    GetIntField
);

impl_value!(
    i64,
    SetStaticLongField,
    GetStaticLongField,
    SetLongField,
    GetLongField
);

impl_value!(
    f32,
    SetStaticFloatField,
    GetStaticFloatField,
    SetFloatField,
    GetFloatField
);

impl_value!(
    f64,
    SetStaticDoubleField,
    GetStaticDoubleField,
    SetDoubleField,
    GetDoubleField
);

impl StaticFieldValue for JObject<'_> {
    fn apply(&self, env: &mut JNIEnv<'_>, c: &JClass<'_>, id: JStaticFieldID) {
        let env = env.get_raw();
        unsafe {
            (**env).SetStaticObjectField.unwrap()(env, c.as_raw(), id.into_raw(), self.as_raw());
        }
    }

    fn get(env: &mut JNIEnv<'_>, c: &JClass<'_>, id: JStaticFieldID) -> Self {
        let env = env.get_raw();
        unsafe {
            let obj = (**env).GetStaticObjectField.unwrap()(env, c.as_raw(), id.into_raw());
            JObject::from_raw(obj)
        }
    }
}

impl FieldValue for JObject<'_> {
    fn apply(&self, env: &mut JNIEnv<'_>, c: &JObject<'_>, id: JFieldID) {
        let env = env.get_raw();
        unsafe {
            (**env).SetObjectField.unwrap()(env, c.as_raw(), id.into_raw(), self.as_raw());
        }
    }

    fn get(env: &mut JNIEnv<'_>, c: &JObject<'_>, id: JFieldID) -> Self {
        let env = env.get_raw();
        unsafe {
            let obj = (**env).GetObjectField.unwrap()(env, c.as_raw(), id.into_raw());
            JObject::from_raw(obj)
        }
    }
}
