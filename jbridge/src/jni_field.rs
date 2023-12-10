use jni::{
    objects::{
        JClass, JFieldID, JList, JMap, JObject, JObjectArray, JPrimitiveArray, JStaticFieldID,
        JString,
    },
    sys::{jboolean, jbyte, jchar, jdouble, jfloat, jint, jlong, jshort},
    JNIEnv,
};

pub trait ApplicableFieldValue<T, ID> {
    fn apply(&self, env: &mut JNIEnv<'_>, c: &T, id: ID);
}

pub trait GettableFieldValue<T, ID> {
    fn get(env: &mut JNIEnv<'_>, c: &T, id: ID) -> Self;
}

macro_rules! impl_primitive {
    ($t:ty, $sm_static:ident, $gm_static:ident, $sm:ident, $gm:ident) => {
        impl ApplicableFieldValue<JClass<'_>, JStaticFieldID> for $t {
            fn apply(&self, env: &mut JNIEnv<'_>, c: &JClass<'_>, id: JStaticFieldID) {
                let raw_env = env.get_raw();
                unsafe {
                    (**raw_env).$sm_static.unwrap()(raw_env, c.as_raw(), id.into_raw(), *self);
                }
            }
        }

        impl GettableFieldValue<JClass<'_>, JStaticFieldID> for $t {
            fn get(env: &mut JNIEnv<'_>, c: &JClass<'_>, id: JStaticFieldID) -> Self {
                let env = env.get_raw();
                unsafe { (**env).$gm_static.unwrap()(env, c.as_raw(), id.into_raw()) }
            }
        }

        impl ApplicableFieldValue<JObject<'_>, JFieldID> for $t {
            fn apply(&self, env: &mut JNIEnv<'_>, c: &JObject<'_>, id: JFieldID) {
                let raw_env = env.get_raw();
                unsafe {
                    (**raw_env).$sm.unwrap()(raw_env, c.as_raw(), id.into_raw(), *self);
                }
            }
        }

        impl GettableFieldValue<JObject<'_>, JFieldID> for $t {
            fn get(env: &mut JNIEnv<'_>, c: &JObject<'_>, id: JFieldID) -> Self {
                let env = env.get_raw();
                unsafe { (**env).$gm.unwrap()(env, c.as_raw(), id.into_raw()) }
            }
        }
    };
}

impl_primitive!(
    jboolean,
    SetStaticBooleanField,
    GetStaticBooleanField,
    SetBooleanField,
    GetBooleanField
);

impl_primitive!(
    i8,
    SetStaticByteField,
    GetStaticByteField,
    SetByteField,
    GetByteField
);

impl_primitive!(
    i16,
    SetStaticShortField,
    GetStaticShortField,
    SetShortField,
    GetShortField
);

impl_primitive!(
    i32,
    SetStaticIntField,
    GetStaticIntField,
    SetIntField,
    GetIntField
);

impl_primitive!(
    i64,
    SetStaticLongField,
    GetStaticLongField,
    SetLongField,
    GetLongField
);

impl_primitive!(
    f32,
    SetStaticFloatField,
    GetStaticFloatField,
    SetFloatField,
    GetFloatField
);

impl_primitive!(
    f64,
    SetStaticDoubleField,
    GetStaticDoubleField,
    SetDoubleField,
    GetDoubleField
);

trait AsJObject<'a>: AsRef<JObject<'a>> {}
trait FromJObject<'a>: From<JObject<'a>> {}

impl<'a, O: AsJObject<'a>> ApplicableFieldValue<JClass<'_>, JStaticFieldID> for O {
    fn apply(&self, env: &mut JNIEnv<'_>, c: &JClass<'_>, id: JStaticFieldID) {
        let env = env.get_raw();
        unsafe {
            (**env).SetStaticObjectField.unwrap()(
                env,
                c.as_raw(),
                id.into_raw(),
                self.as_ref().as_raw(),
            );
        }
    }
}

impl<'a, O: FromJObject<'a>> GettableFieldValue<JClass<'_>, JStaticFieldID> for O {
    fn get(env: &mut JNIEnv<'_>, c: &JClass<'_>, id: JStaticFieldID) -> Self {
        let env = env.get_raw();
        unsafe {
            let obj = (**env).GetStaticObjectField.unwrap()(env, c.as_raw(), id.into_raw());
            JObject::from_raw(obj).into()
        }
    }
}

impl<'a, O: AsJObject<'a>> ApplicableFieldValue<JObject<'_>, JFieldID> for O {
    fn apply(&self, env: &mut JNIEnv<'_>, c: &JObject<'_>, id: JFieldID) {
        let env = env.get_raw();
        unsafe {
            (**env).SetObjectField.unwrap()(env, c.as_raw(), id.into_raw(), self.as_ref().as_raw());
        }
    }
}

impl<'a, O: FromJObject<'a>> GettableFieldValue<JObject<'_>, JFieldID> for O {
    fn get(env: &mut JNIEnv<'_>, c: &JObject<'_>, id: JFieldID) -> Self {
        let env = env.get_raw();
        unsafe {
            let obj = (**env).GetObjectField.unwrap()(env, c.as_raw(), id.into_raw());
            JObject::from_raw(obj).into()
        }
    }
}

macro_rules! impl_object_like {
    ($($t:ty,)*) => {
        $(
            impl<'a> AsJObject<'a> for $t {}
            impl<'a> FromJObject<'a> for $t {}
        )*
    };
}

impl_object_like!(
    JClass<'a>,
    JObject<'a>,
    JString<'a>,
    JObjectArray<'a>,
    JPrimitiveArray<'a, jboolean>,
    JPrimitiveArray<'a, jchar>,
    JPrimitiveArray<'a, jshort>,
    JPrimitiveArray<'a, jbyte>,
    JPrimitiveArray<'a, jlong>,
    JPrimitiveArray<'a, jint>,
    JPrimitiveArray<'a, jdouble>,
    JPrimitiveArray<'a, jfloat>,
);

impl<'a, 'b, 'c> AsJObject<'a> for JList<'b, 'a, 'c> {}
impl<'a, 'b, 'c> AsJObject<'a> for JMap<'b, 'a, 'c> {}
