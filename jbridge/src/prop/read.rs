use std::{borrow::Cow, marker::PhantomData, sync::OnceLock};

use jni::{
    errors::{Error, Result as JResult},
    objects::{JFieldID, JMethodID, JObject, JValueGen},
    signature::ReturnType,
    JNIEnv,
};

use crate::{jni_error::ErrorExt, JavaMethodDescription, JavaTypeDescription};

pub enum PropertyRead<T> {
    Field(FieldPropertyRead<T>),
    Getter(FieldMethodRead<T>),
}

impl<T> PropertyRead<T>
where
    T: for<'a> TryFrom<JValueGen<JObject<'a>>, Error = Error>,
{
    pub fn new_field(field_name: Cow<'static, str>, sig: Cow<'static, str>) -> Self {
        Self::Field(FieldPropertyRead::new(field_name, sig))
    }

    pub fn new_getter(name: Cow<'static, str>, sig: Cow<'static, str>) -> Self {
        Self::Getter(FieldMethodRead::new(name, sig))
    }

    pub fn access<'local>(&self, env: &mut JNIEnv<'local>, obj: &JObject<'local>) -> JResult<T> {
        match self {
            PropertyRead::Field(f) => f.access(env, obj),
            PropertyRead::Getter(g) => g.access(env, obj),
        }
    }
}

pub struct FieldPropertyRead<T> {
    field_name: Cow<'static, str>,
    sig: Cow<'static, str>,

    field_id: OnceLock<JResult<(JFieldID, ReturnType)>>,
    _phantom: PhantomData<T>,
}

impl<T> FieldPropertyRead<T>
where
    T: for<'a> TryFrom<JValueGen<JObject<'a>>, Error = Error>,
{
    fn new(field_name: Cow<'static, str>, sig: Cow<'static, str>) -> Self {
        Self {
            field_name,
            sig,
            field_id: OnceLock::new(),
            _phantom: PhantomData,
        }
    }

    fn access<'local>(&self, env: &mut JNIEnv<'local>, obj: &JObject<'local>) -> JResult<T> {
        let (field_id, return_type) = match self.field_id.get_or_init(|| {
            Ok((
                env.get_field_id(
                    env.auto_local(env.get_object_class(obj)?),
                    self.field_name.as_ref(),
                    self.sig.as_ref(),
                )?,
                JavaTypeDescription::try_from(self.sig.as_ref())
                    .map_err(|_| Error::JavaVMMethodNotFound("Unable to parse field signature"))?
                    .into(),
            ))
        }) {
            Ok(id) => (id.0.clone(), id.1.clone()),
            Err(e) => return Err(e.cloned()),
        };

        env.get_field_unchecked(obj, field_id, return_type)?
            .try_into()
    }
}

pub struct FieldMethodRead<T> {
    sig: Cow<'static, str>,
    name: Cow<'static, str>,
    method_id: OnceLock<JResult<(JMethodID, ReturnType)>>,
    _phantom: PhantomData<T>,
}

impl<T> FieldMethodRead<T>
where
    T: for<'a> TryFrom<JValueGen<JObject<'a>>, Error = Error>,
{
    fn new(name: Cow<'static, str>, sig: Cow<'static, str>) -> Self {
        Self {
            sig,
            name,
            method_id: OnceLock::new(),
            _phantom: PhantomData,
        }
    }

    fn access<'local>(&self, env: &mut JNIEnv<'local>, obj: &JObject<'local>) -> JResult<T> {
        let (method_id, ret_type) = match self.method_id.get_or_init(|| {
            Ok((
                env.get_method_id(
                    env.auto_local(env.get_object_class(obj)?),
                    self.name.as_ref(),
                    self.sig.as_ref(),
                )?,
                JavaMethodDescription::try_from(self.sig.as_ref())
                    .map_err(|_| Error::JavaVMMethodNotFound("Unable to parse method signature"))?
                    .return_type
                    .into(),
            ))
        }) {
            Ok(id) => (id.0.clone(), id.1.clone()),
            Err(e) => return Err(e.cloned()),
        };

        unsafe { env.call_method_unchecked(obj, method_id, ret_type, &[]) }?.try_into()
    }
}
