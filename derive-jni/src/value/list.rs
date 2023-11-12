use std::{collections::VecDeque, fmt::Display};

use jni::{
    objects::{JList, JObject},
    JNIEnv,
};

use crate::ToJavaValue;

#[derive(thiserror::Error)]
pub enum Error<E: Display> {
    #[error("Error converting to Java type: {0}")]
    Convert(E),
    #[error("Error with JNI call: {0}")]
    Java(#[from] jni::errors::Error),
}

macro_rules! impl_list_like {
    ($container:ty) => {
        impl<T: ToJavaValue> ToJavaValue for $container {
            type JavaType<'a> = JObject<'a>;
            type ConvertError = Error<T::BoxingError>;
            type BoxingError = Error<T::BoxingError>;

            fn into_java_value<'s>(
                &self,
                env: &mut JNIEnv<'s>,
            ) -> Result<Self::JavaType<'s>, Self::ConvertError> {
                iter_to_java_list(env, self.iter())
            }

            fn into_java_value_boxed<'s>(
                &self,
                env: &mut JNIEnv<'s>,
            ) -> Result<JObject<'s>, Self::BoxingError> {
                self.into_java_value(env)
            }
        }
    };
}

impl_list_like!(&[T]);
impl_list_like!(Vec<T>);
impl_list_like!(VecDeque<T>);

fn iter_to_java_list<'s, 'a, I>(
    env: &mut JNIEnv<'s>,
    iter: impl Iterator<Item = &'a I>,
) -> Result<JObject<'s>, Error<I::BoxingError>>
where
    I: ToJavaValue + 'a,
{
    let list = match iter.size_hint().1 {
        Some(max) => env.new_object("java/util/ArrayList", "(I)V", &[(max as i32).into()])?,
        None => env.new_object("java/util/ArrayList", "()V", &[])?,
    };

    let jlist = JList::from_env(env, &list)?;
    for v in iter {
        let v = v
            .into_java_value_boxed(env)
            .map_err(|e| Error::Convert(e))?;
        jlist.add(env, &v)?;
    }

    Ok(list)
}
