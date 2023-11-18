use jni::sys::JNIEnv;
use reactive_core::{ContextKey, SetupContext, Signal};
use reactive_derive::component;

static JNI_CONTEXT_KEY: ContextKey<JNIEnv> = ContextKey::new();

#[component]
pub fn text(ctx: &mut SetupContext, text: impl Signal<Value = String>) {
    ctx.create_effect_fn(move |ctx| {
        let env = ctx.get(&JNI_CONTEXT_KEY).unwrap();
        let text = text.get();
        let text = env.new_string(text).unwrap();
        let text = text.into_inner();
        println!("text: {}", text);
    });
}
