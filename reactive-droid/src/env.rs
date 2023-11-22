use std::{cell::RefCell, ptr::null_mut};

use jni::JNIEnv;

thread_local! {
    static CURRENT_JNI_ENV: RefCell<*mut jni::sys::JNIEnv> = RefCell::new(null_mut());
}

pub fn with_current_jni_env<'local, T>(cb: impl FnOnce(JNIEnv<'local>) -> T) -> Option<T> {
    CURRENT_JNI_ENV.with(|env| {
        let env = unsafe { JNIEnv::from_raw(*env.borrow()) }.ok()?;
        Some(cb(env))
    })
}

pub fn set_current_jni_env<T>(env: &JNIEnv<'_>, f: impl FnOnce() -> T) -> T {
    CURRENT_JNI_ENV.replace(env.get_native_interface());
    let result = f();
    CURRENT_JNI_ENV.replace(null_mut());
    result
}
