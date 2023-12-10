use std::{borrow::Cow, marker::PhantomData, sync::OnceLock};

use jni::{
    errors::Result,
    objects::{JFieldID, JMethodID, JObject, JValueGen, JValueOwned},
    JNIEnv,
};

use crate::{GettableFieldValue, JavaTypeDescription};

pub trait PropertyAccess {
    fn get_property<'local>(
        &self,
        env: &mut JNIEnv<'local>,
        obj: &JObject<'local>,
        name: &str,
    ) -> Result<JValueOwned<'local>>;

    fn set_property<'local>(
        &mut self,
        env: &mut JNIEnv<'local>,
        obj: &JObject<'local>,
        name: &str,
        value: &JValueGen<JObject<'local>>,
    ) -> Result<()>;
}

pub enum PropertyRead<T> {
    Field(FieldPropertyRead<T>),
    Getter(JMethodID, PhantomData<T>),
}

pub struct FieldPropertyRead<T> {
    sig: Cow<'static, str>,
    name: Cow<'static, str>,
    field_id: OnceLock<Option<JFieldID>>,
    _phantom: PhantomData<T>,
}

impl<T> FieldPropertyRead<T>
where
    T: for<'a> GettableFieldValue<JObject<'a>, JFieldID>,
{
    pub fn new(sig: Cow<'static, str>, name: Cow<'static, str>) -> Self {
        Self {
            field_id: OnceLock::new(),
            sig,
            name,
            _phantom: PhantomData,
        }
    }

    fn access<'local>(&self, env: &mut JNIEnv<'local>, obj: &JObject<'local>) -> Result<T> {
        let field_id = self.field_id.get_or_init(|| {
            env.get_object_class(obj)
                .and_then(|c| env.get_field_id(&c, &self.name, &self.sig))
                .ok()
        });

        let Some(field_id) = field_id else {
            return Err(::jni::errors::Error::FieldNotFound {
                name: self.name.to_string(),
                sig: self.sig.to_string(),
            });
        };

        Ok(T::get(env, obj, *field_id))
    }
}

pub struct FieldMethodRead<T> {
    sig: Cow<'static, str>,
    name: Cow<'static, str>,
    method_id: OnceLock<Option<JMethodID>>,
    _phantom: PhantomData<T>,
}


