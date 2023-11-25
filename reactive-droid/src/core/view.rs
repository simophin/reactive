use std::borrow::Cow;

use crate::env::with_current_android_runtime;
use crate::value::IntoJValue;
use derive_builder::Builder;
use jni::objects::GlobalRef;
use jni::{
    objects::{JObject, JValueGen},
    JNIEnv,
};
use reactive_core::core_component::ProviderBuilder;
use reactive_core::{Component, ContextKey, EffectContext, SetupContext, Signal, SingleValue};

pub static ANDROID_VIEW_CONTAINER_KEY: ContextKey<AndroidViewContainer> = ContextKey::new();

pub enum AndroidViewContainer {
    ViewParent(GlobalRef),
    Activity(GlobalRef),
    Empty,
}

#[derive(Builder)]
#[builder(pattern = "owned")]
pub struct AndroidView {
    #[builder(setter(into))]
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

        let parent = ctx.use_context(&ANDROID_VIEW_CONTAINER_KEY);

        let provider_value = if auto_adopt_child
            && matches!(
                with_current_android_runtime(|rt| {
                    rt.env().is_instance_of(&obj, "android.view.ViewParent")
                }),
                Some(Ok(true))
            ) {
            AndroidViewContainer::ViewParent(obj.clone())
        } else {
            AndroidViewContainer::Empty
        };

        ctx.children.push(Box::new(
            ProviderBuilder::default()
                .key(&ANDROID_VIEW_CONTAINER_KEY)
                .value(SingleValue(provider_value))
                .child(children)
                .build()
                .unwrap(),
        ));

        if let Some(parent) = parent {
            with_current_android_runtime(|rt| {
                parent.with(|p| match p {
                    AndroidViewContainer::ViewParent(p) => {
                        if let Err(e) = rt.env().call_method(
                            p,
                            "addView",
                            "(Landroid/view/View;)V",
                            &[(&obj).into()],
                        ) {
                            log::error!("Failed to add view to parent: {}", e);
                        }
                    }
                    AndroidViewContainer::Activity(p) => {
                        if let Err(e) = rt.env().call_method(
                            p,
                            "setContentView",
                            "(Landroid/view/View;)V",
                            &[(&obj).into()],
                        ) {
                            log::error!("Failed to add view to parent: {}", e);
                        }
                    }
                    AndroidViewContainer::Empty => {}
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
    pub fn property<V>(
        self,
        java_method_name: impl AsRef<str> + 'static,
        java_signature: impl AsRef<str> + 'static,
        value: impl Signal<Value = V>,
    ) -> Self
    where
        V: IntoJValue,
    {
        let mut property_runs = self.property_runs.unwrap_or_default();

        property_runs.push(Box::new(move |_ctx, env, obj| {
            value.with(|value| {
                let value = value
                    .into_jvalue(env)
                    .expect("To convert value to Java value");

                let value = match &value {
                    JValueGen::Object(v) => JValueGen::Object(v.as_ref()),
                    JValueGen::Bool(v) => JValueGen::Bool(*v),
                    JValueGen::Byte(v) => JValueGen::Byte(*v),
                    JValueGen::Char(v) => JValueGen::Char(*v),
                    JValueGen::Short(v) => JValueGen::Short(*v),
                    JValueGen::Int(v) => JValueGen::Int(*v),
                    JValueGen::Long(v) => JValueGen::Long(*v),
                    JValueGen::Float(v) => JValueGen::Float(*v),
                    JValueGen::Double(v) => JValueGen::Double(*v),
                    JValueGen::Void => JValueGen::Void,
                };

                if let Err(e) = env.call_method(
                    obj,
                    java_method_name.as_ref(),
                    java_signature.as_ref(),
                    &[value],
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
