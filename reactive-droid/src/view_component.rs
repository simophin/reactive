use jni::objects::{GlobalRef, JObject};
use jni::JNIEnv;
use reactive_core::{Component, SetupContext, UserDataKey};

use crate::env::with_current_jni_env;

pub trait ViewComponent {
    fn setup<'a>(
        self: Self,
        ctx: &mut SetupContext,
        env: &mut JNIEnv<'a>,
    ) -> jni::errors::Result<JObject<'a>>;
}

impl<F> ViewComponent for F
where
    F: for<'a> FnOnce(&mut SetupContext, &mut JNIEnv<'a>) -> jni::errors::Result<JObject<'a>>
        + 'static,
{
    fn setup<'a>(
        self: Self,
        ctx: &mut SetupContext,
        env: &mut JNIEnv<'a>,
    ) -> jni::errors::Result<JObject<'a>> {
        self(ctx, env)
    }
}

pub(crate) static ANDROID_VIEW_DATA_KEY: UserDataKey<GlobalRef> = UserDataKey::new();

pub fn android_view(vc: impl ViewComponent + 'static) -> impl Component {
    move |ctx: &mut SetupContext| {
        let view = match with_current_jni_env(|mut env| {
            vc.setup(ctx, &mut env)
                .and_then(|obj| env.new_global_ref(obj))
        }) {
            Some(Ok(view)) => view,
            Some(Err(err)) => {
                log::error!("Failed to setup view: {}", err);
                return;
            }
            None => {
                log::error!("No JNI environment available to set up context");
                return;
            }
        };

        ctx.set_user_data(&ANDROID_VIEW_DATA_KEY, view);
    }
}
