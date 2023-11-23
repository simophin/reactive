use std::{cell::RefCell, ptr::null_mut};

use jni::JNIEnv;
use jni::objects::JObject;

thread_local! {
    static CURRENT_ENV: RefCell<*mut JavaRuntimeEnv> = RefCell::new(null_mut());
}

struct JavaRuntimeEnv<'a> {
    pub env: JNIEnv<'a>,
    pub activity: JObject<'a>,
}

pub fn with_current_java_env<'local, T>(cb: impl FnOnce(JNIEnv<'local>) -> T) -> Option<T> {
    CURRENT_ENV.with(|env| {
        let env = unsafe { JNIEnv::from_raw(*env.borrow()) }.ok()?;
        Some(cb(env))
    })
}

pub fn set_current_java_env<T>(env: &JNIEnv<'_>, f: impl FnOnce() -> T) -> T {
    CURRENT_ENV.replace(env.get_native_interface());
    let result = f();
    CURRENT_ENV.replace(null_mut());
    result
}
