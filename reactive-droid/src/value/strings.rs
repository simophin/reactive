use jni::{
    errors::Result,
    objects::{AutoLocal, JObject, JValueGen},
    JNIEnv,
};

use super::IntoJValue;

impl IntoJValue for str {
    fn into_jvalue<'local>(
        &self,
        env: &mut JNIEnv<'local>,
    ) -> Result<JValueGen<AutoLocal<'local, JObject<'local>>>> {
        let value = env.new_string(self)?;
        Ok(JValueGen::Object(AutoLocal::new(value.into(), env)))
    }
}

impl IntoJValue for String {
    fn into_jvalue<'local>(
        &self,
        env: &mut JNIEnv<'local>,
    ) -> Result<JValueGen<AutoLocal<'local, JObject<'local>>>> {
        self.as_str().into_jvalue(env)
    }
}
