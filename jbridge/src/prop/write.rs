use std::{borrow::Cow, marker::PhantomData, sync::OnceLock};

use jni::{
    errors::{Error, Result as JResult},
    objects::{JFieldID, JMethodID, JObject, JValue, JValueGen},
    signature::ReturnType,
    JNIEnv,
};

use crate::{jni_error::ErrorExt, JavaMethodDescription};

pub enum PropertyWriter<T> {
    Field(FieldPropertyWrite<T>),
    Setter(SetterProperty<T>),
}

impl<T> PropertyWriter<T>
where
    T: for<'a, 'b> Into<JValueGen<&'b JObject<'a>>>,
{
    pub fn new_field(field_name: Cow<'static, str>, sig: Cow<'static, str>) -> Self {
        Self::Field(FieldPropertyWrite {
            field_name,
            sig,
            field_id: OnceLock::new(),
            _phantom: PhantomData,
        })
    }

    pub fn new_setter(name: Cow<'static, str>, sig: Cow<'static, str>) -> Self {
        Self::Setter(SetterProperty {
            method_name: name,
            sig,
            method_id: OnceLock::new(),
            _phantom: PhantomData,
        })
    }

    pub fn write<'local>(
        &self,
        env: &mut JNIEnv<'local>,
        obj: &JObject<'local>,
        value: T,
    ) -> JResult<()> {
        match self {
            PropertyWriter::Field(f) => f.write(env, obj, value),
            PropertyWriter::Setter(s) => s.write(env, obj, value),
        }
    }
}

pub struct FieldPropertyWrite<T> {
    field_name: Cow<'static, str>,
    sig: Cow<'static, str>,

    field_id: OnceLock<JResult<JFieldID>>,
    _phantom: PhantomData<T>,
}

impl<T> FieldPropertyWrite<T>
where
    T: for<'a, 'b> Into<JValueGen<&'b JObject<'a>>>,
{
    fn write<'local>(
        &self,
        env: &mut JNIEnv<'local>,
        obj: &JObject<'local>,
        value: T,
    ) -> JResult<()> {
        let field_id = match self.field_id.get_or_init(|| {
            env.get_field_id(env.get_object_class(obj)?, &self.field_name, &self.sig)
        }) {
            Ok(v) => *v,
            Err(e) => return Err(e.cloned()),
        };

        env.set_field_unchecked(obj, field_id, value.into())
    }
}

pub struct SetterProperty<T> {
    method_name: Cow<'static, str>,
    sig: Cow<'static, str>,

    method_id: OnceLock<JResult<(JMethodID, ReturnType)>>,
    _phantom: PhantomData<T>,
}

impl<T> SetterProperty<T>
where
    T: for<'a, 'b> Into<JValueGen<&'b JObject<'a>>>,
{
    fn write<'local>(
        &self,
        env: &mut JNIEnv<'local>,
        obj: &JObject<'local>,
        value: T,
    ) -> JResult<()> {
        let (method_id, return_type) = match self.method_id.get_or_init(|| {
            Ok((
                env.get_method_id(env.get_object_class(obj)?, &self.method_name, &self.sig)?,
                JavaMethodDescription::try_from(self.sig.as_ref())
                    .map_err(|_| Error::JavaVMMethodNotFound("Invalid signature"))?
                    .return_type
                    .into(),
            ))
        }) {
            Ok(v) => (v.0.clone(), v.1.clone()),
            Err(e) => return Err(e.cloned()),
        };

        let value: JValue = value.into();
        unsafe { env.call_method_unchecked(obj, method_id, return_type, &[value.as_jni()]) }?;
        Ok(())
    }
}
