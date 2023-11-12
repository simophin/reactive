use std::{
    collections::{BTreeMap, HashMap},
    fmt::Debug,
};

use jni::{
    objects::{JMap, JObject},
    JNIEnv,
};

use crate::ToJavaValue;

macro_rules! impl_map_like {
    ($map_type:ident) => {
        impl<K, V> ToJavaValue for $map_type<K, V>
        where
            K: for<'a> ToJavaValue<JavaType<'a> = JObject<'a>>,
            V: for<'a> ToJavaValue<JavaType<'a> = JObject<'a>>,
        {
            type ConvertError = Error<K::BoxingError, V::BoxingError>;
            type BoxingError = Error<K::BoxingError, V::BoxingError>;
            type JavaType<'a> = JObject<'a>;

            fn into_java_value<'s>(
                &self,
                env: &mut JNIEnv<'s>,
            ) -> Result<Self::JavaType<'s>, Self::ConvertError> {
                iter_to_java_map(env, self.iter())
            }

            fn into_java_value_boxed<'s>(
                &self,
                env: &mut JNIEnv<'s>,
            ) -> Result<JObject<'s>, Self::BoxingError> {
                self.into_java_value(env)
            }

            fn java_signature() -> std::borrow::Cow<'static, str> {
                Self::boxed_java_signature()
            }

            fn boxed_java_signature() -> std::borrow::Cow<'static, str> {
                std::borrow::Cow::Borrowed("Ljava/util/Map;")
            }
        }
    };
}

#[derive(thiserror::Error, Debug)]
pub enum Error<KE: std::error::Error, VE: std::error::Error> {
    #[error("Key conversion failed: {0}")]
    Key(KE),
    #[error("Value conversion failed: {0}")]
    Value(VE),
    #[error("Error with JNI call: {0}")]
    Java(#[from] jni::errors::Error),
}

fn iter_to_java_map<'s, 'i, K, V, I>(
    env: &mut JNIEnv<'s>,
    iter: I,
) -> Result<JObject<'s>, Error<K::BoxingError, V::BoxingError>>
where
    K: for<'a> ToJavaValue<JavaType<'a> = JObject<'a>> + 'i,
    V: for<'a> ToJavaValue<JavaType<'a> = JObject<'a>> + 'i,
    I: Iterator<Item = (&'i K, &'i V)>,
{
    let map = env.new_object("java/util/HashMap", "()V", &[])?;
    let jmap = JMap::from_env(env, &map)?;
    for (k, v) in iter {
        let k = k.into_java_value_boxed(env).map_err(|e| Error::Key(e))?;
        let v = v.into_java_value_boxed(env).map_err(|e| Error::Value(e))?;
        jmap.put(env, &k, &v)?;
    }
    Ok(map)
}

impl_map_like!(HashMap);
impl_map_like!(BTreeMap);
