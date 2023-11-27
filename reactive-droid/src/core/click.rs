use std::cell::RefCell;

use jni::{
    objects::{AutoLocal, JObject},
    JNIEnv,
};

use crate::proxy::{new_java_proxy, ProxyHandle};

pub struct OnClickHandler(pub(crate) Box<RefCell<dyn FnMut() + 'static>>);

impl<F> From<F> for OnClickHandler
where
    F: FnMut() + 'static,
{
    fn from(f: F) -> Self {
        Self(Box::new(RefCell::new(f)))
    }
}

impl OnClickHandler {
    fn call(&self) {
        let mut handler = self.0.borrow_mut();
        (*handler)();
    }
}

unsafe impl Send for OnClickHandler {}
unsafe impl Sync for OnClickHandler {}

impl OnClickHandler {
    pub fn to_java_proxy<'local>(
        self,
        env: &mut JNIEnv<'local>,
    ) -> jni::errors::Result<(AutoLocal<'local, JObject<'local>>, ProxyHandle)> {
        new_java_proxy(
            env,
            "android.view.View$OnClickListener",
            Box::new(move |_env, _this, method, _args| {
                if method == "onClick" {
                    self.call();
                }
                JObject::null()
            }),
        )
    }
}
