use jni::{objects::GlobalRef, sys::JNIEnv};
use reactive_core::{ContextKey, SetupContext, Signal, UserDataKey};
use reactive_derive::component;


#[component]
pub fn text(ctx: &mut SetupContext, text: impl Signal<Value = String>) {
    ctx.set_user_data(&ANDROID_VIEW_DATA_KEY, value)
}
