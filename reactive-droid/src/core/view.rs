use std::borrow::Cow;

use derive_builder::Builder;
use derive_jni::ToJavaValue;
use jni::{
    objects::{JObject, JValueGen},
    JNIEnv,
};
use jni::objects::GlobalRef;
use jni::sys::jobject;
use reactive_core::{Component, ContextKey, EffectContext, SetupContext, Signal};
use reactive_core::core_component::{Provider, ProviderBuilder};
use crate::env::with_current_java_env;

pub static ANDROID_CONTEXT_KEY: ContextKey<jobject> = ContextKey::new();
pub static ANDROID_VIEW_KEY: ContextKey<GlobalRef> = ContextKey::new();

#[derive(Builder)]
#[builder(pattern = "owned")]
pub struct AndroidView {
    class_name: Cow<'static, str>,
    #[builder(setter(custom, strip_option))]
    property_runs:
    Vec<Box<dyn for<'a> Fn(&'a mut EffectContext, &'a mut JNIEnv<'_>, &'a JObject<'_>)>>,
    children: Vec<Box<dyn Component>>,
}

impl Component for AndroidView {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let obj = with_current_java_env(|mut env| {

            env.new_object(self.class_name, "(Landroid/content/Context;)V", &[])
                .and_then(|o| env.new_global_ref(o))
        });

        let obj = match obj {
            Some(Ok(v)) => v,
            Some(Err(e)) => log::error!("Failed to create AndroidView: {}", e),
            None => log::error!("Failed to create AndroidView: No JNIEnv"),
        };

        let parent = ctx.use_context(&ANDROID_VIEW_KEY);

        ProviderBuilder::default().key(&ANDROID_VIEW_KEY).value(obj).child(self.children).build()
            .unwrap()
            .setup(ctx);

        for run in self.property_runs {
            let obj = obj.clone();
            ctx.create_effect(move |ctx, env| {
                with_current_java_env(|mut env| {
                    run(ctx, &mut env, obj.as_obj());
                });
            });
        }
    }
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
