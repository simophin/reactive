//! `com.reactive.NativeCallback`: a Java listener that forwards to a Rust
//! closure. The closure is freed by `release()`, which the owning component
//! calls on cleanup.

use super::app::register;
use super::java::JavaObject;
use jni::JNIEnv;
use jni::objects::{JClass, JValue};
use jni::sys::jlong;
use std::ffi::c_void;

type Callback = Box<dyn Fn()>;

pub(crate) const CLASS: &str = "com/reactive/NativeCallback";

/// Creates a `NativeCallback` running `f` on the main thread.
pub(crate) fn new_callback(f: impl Fn() + 'static) -> JavaObject {
    let handle = Box::into_raw(Box::new(Box::new(f) as Callback));
    JavaObject::construct(CLASS, "(J)V", &[JValue::Long(handle as jlong)])
}

pub(crate) fn register_natives(env: &mut JNIEnv) {
    register(
        env,
        CLASS,
        &[
            ("nativeInvoke", "(J)V", native_invoke as *mut c_void),
            ("nativeRelease", "(J)V", native_release as *mut c_void),
        ],
    );
}

extern "system" fn native_invoke(_env: JNIEnv, _class: JClass, handle: jlong) {
    let callback = unsafe { &*(handle as *const Callback) };
    callback();
}

extern "system" fn native_release(_env: JNIEnv, _class: JClass, handle: jlong) {
    drop(unsafe { Box::from_raw(handle as *mut Callback) });
}
