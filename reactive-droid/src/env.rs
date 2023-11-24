use std::{cell::RefCell, ptr::null_mut};

use jni::objects::JObject;
use jni::JNIEnv;

thread_local! {
    static CURRENT_ENV: RefCell<Option<AndroidRuntime>> = Default::default();
}

pub struct AndroidRuntime {
    env: *mut jni::sys::JNIEnv,
    activity: jni::sys::jobject,
}

impl AndroidRuntime {
    pub fn new(env: &JNIEnv<'_>, activity: &JObject<'_>) -> Self {
        assert_ne!(env.get_native_interface(), null_mut());
        Self {
            env: env.get_native_interface(),
            activity: activity.as_raw(),
        }
    }

    pub fn activity(&self) -> JObject<'_> {
        unsafe { JObject::from_raw(self.activity) }
    }

    pub fn env(&self) -> JNIEnv<'_> {
        unsafe { JNIEnv::from_raw(self.env).unwrap() }
    }
}

pub fn with_current_android_runtime<'local, T>(cb: impl FnOnce(&AndroidRuntime) -> T) -> Option<T> {
    CURRENT_ENV.with(|env| env.borrow().as_ref().map(|runtime| cb(runtime)))
}

pub fn set_current_android_runtime<T>(runtime: AndroidRuntime, cb: impl FnOnce() -> T) -> T {
    CURRENT_ENV.replace(Some(runtime));
    let r = cb();
    CURRENT_ENV.replace(None);
    r
}
