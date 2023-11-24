use std::borrow::Cow;

use crate::env::with_current_android_runtime;
use derive_builder::Builder;
use derive_jni::ToJavaValue;
use jni::objects::GlobalRef;
use jni::{
    objects::{JObject, JValueGen},
    JNIEnv,
};
use reactive_core::core_component::ProviderBuilder;
use reactive_core::{Component, ContextKey, EffectContext, SetupContext, Signal, SingleValue};

pub static ANDROID_VIEW_KEY: ContextKey<Option<GlobalRef>> = ContextKey::new();

#[derive(Builder)]
#[builder(pattern = "owned")]
pub struct AndroidView {
    class_name: Cow<'static, str>,
    #[builder(setter(custom, strip_option))]
    property_runs:
        Vec<Box<dyn for<'a> Fn(&'a mut EffectContext, &'a mut JNIEnv<'_>, &'a JObject<'_>)>>,
    children: Vec<Box<dyn Component>>,
    auto_adopt_child: bool,
}

impl Component for AndroidView {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let Self {
            class_name,
            property_runs,
            children,
            auto_adopt_child,
        } = *self;

        let obj = with_current_android_runtime(move |rt| {
            let mut env = rt.env();
            env.new_object(
                class_name,
                "(Landroid/content/Context;)V",
                &[(&rt.activity()).into()],
            )
            .and_then(|o| env.new_global_ref(o))
        });

        let obj = match obj {
            Some(Ok(v)) => v,
            Some(Err(e)) => {
                log::error!("Failed to create AndroidView: {}", e);
                return;
            }
            None => {
                log::error!("Failed to create AndroidView: No JNIEnv");
                return;
            }
        };

        let parent = ctx.use_context(&ANDROID_VIEW_KEY);

        let provider_value = if auto_adopt_child
            && matches!(
                with_current_android_runtime(|rt| {
                    rt.env().is_instance_of(&obj, "android.view.ViewGroup")
                }),
                Some(Ok(true))
            ) {
            Some(obj.clone())
        } else {
            None
        };

        Box::new(
            ProviderBuilder::default()
                .key(&ANDROID_VIEW_KEY)
                .value(SingleValue(provider_value))
                .child(children)
                .build()
                .unwrap(),
        )
        .setup(ctx);

        if let Some(parent) = parent {
            with_current_android_runtime(|rt| {
                parent.with(|p| {
                    let Some(p) = p else {
                        return;
                    };

                    if let Err(e) = rt.env().call_method(
                        p,
                        "addView",
                        "(Landroid/view/View;)V",
                        &[(&obj).into()],
                    ) {
                        log::error!("Failed to add view to parent: {}", e);
                    }
                })
            });
        }

        for run in property_runs {
            let obj = obj.clone();
            ctx.create_effect_fn(move |ctx| {
                with_current_android_runtime(|rt| {
                    run(ctx, &mut rt.env(), &obj);
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
        let mut property_runs = self.property_runs.unwrap_or_default();

        property_runs.push(Box::new(move |_ctx, env, obj| {
            value.with(|value| {
                let value = value
                    .into_java_value(env)
                    .expect("To convert value to Java value");

                if let Err(e) = env.call_method(
                    obj,
                    java_method_name.as_ref(),
                    java_signature.as_ref(),
                    &[value.into()],
                ) {
                    log::error!(
                        "Failed to set property {}: {e:?}",
                        java_method_name.as_ref(),
                    );
                }
            });
        }));

        Self {
            property_runs: Some(property_runs),
            ..self
        }
    }
}
