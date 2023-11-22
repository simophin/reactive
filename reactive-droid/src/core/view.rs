use std::borrow::Cow;

use derive_builder::Builder;
use derive_jni::ToJavaValue;
use jni::{
    objects::{JObject, JValueGen},
    JNIEnv,
};
use reactive_core::{EffectContext, Signal};

#[derive(Builder)]
#[builder(pattern = "owned")]
pub struct AndroidView {
    class_name: Cow<'static, str>,
    #[builder(setter(custom, strip_option))]
    property_runs:
        Vec<Box<dyn for<'a> Fn(&'a mut EffectContext, &'a mut JNIEnv<'_>, &'a JObject<'_>)>>,
}

impl AndroidViewBuilder {
    pub fn property<S>(
        self,
        java_method_name: impl AsRef<str> + 'static,
        java_signature: impl AsRef<str> + 'static,
        value: S,
    ) -> Self
    where
        S: Signal,
        S::Value: ToJavaValue,
        for<'a> <S::Value as ToJavaValue>::JavaType<'a>: Into<JValueGen<&'a JObject<'a>>>,
    {
        self.property_runs.push(Box::new(move |ctx, env, obj| {
            value.with(|value| {
                let value = value
                    .into_java_value(env)
                    .expect("To convert value to Java value");

                env.call_method(
                    obj,
                    java_method_name.as_ref(),
                    java_signature.as_ref(),
                    &[value.into()],
                );
            });
        }));
        self
    }
}
