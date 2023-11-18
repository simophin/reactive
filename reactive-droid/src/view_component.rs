use jni::JNIEnv;
use jni::objects::{GlobalRef, JObject};
use reactive_core::{Component, SetupContext};

pub trait ViewComponent {
    fn setup<'a>(self: Box<Self>, ctx: &mut SetupContext, env: &mut JNIEnv<'a>) -> jni::errors::Result<JObject<'a>>;

    fn into_component(self);
}

pub struct ViewComponentWrapper<VC>(VC);

impl<VC: ViewComponent> Component for ViewComponentWrapper<VC> {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        todo!()
    }
}
