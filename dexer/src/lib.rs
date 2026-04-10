pub mod class_def;
pub mod output;
mod writer;

pub use dexer_macros::dex_class;

pub use class_def::{AccessFlags, ClassDef, FieldDef, MethodCode, MethodEntry};
pub use output::{DexOutput, NativeMethod, NativeRegistrations};
use std::cell::Cell;
use std::mem::ManuallyDrop;

thread_local! {
    static CURRENT_THIS: Cell<jni::sys::jobject> = const { Cell::new(std::ptr::null_mut()) };
}

pub struct CurrentThisGuard {
    previous: jni::sys::jobject,
}

pub fn push_current_this(this_raw: jni::sys::jobject) -> CurrentThisGuard {
    let previous = CURRENT_THIS.with(|cell| cell.replace(this_raw));
    CurrentThisGuard { previous }
}

impl Drop for CurrentThisGuard {
    fn drop(&mut self) {
        CURRENT_THIS.with(|cell| cell.set(self.previous));
    }
}

pub fn current_this<'local>(
    env: &mut jni::JNIEnv<'local>,
) -> jni::errors::Result<jni::objects::JObject<'local>> {
    let raw = CURRENT_THIS.with(|cell| cell.get());
    if raw.is_null() {
        return Err(jni::errors::Error::NullPtr("current_this".into()));
    }

    let obj = ManuallyDrop::new(unsafe { jni::objects::JObject::from_raw(raw) });
    env.new_local_ref(&*obj).map(jni::objects::JObject::from)
}

/// Passed to `#[override]` method implementations so they can call the superclass method.
/// Stores the raw `this` pointer and the generated `$$super` method name/descriptor.
/// Receives `env` at call-time to avoid lifetime entanglement with the bridge frame.
pub struct SuperCaller {
    this_raw: jni::sys::jobject,
    method_name: &'static str,
    descriptor: &'static str,
}

impl SuperCaller {
    /// Called by generated bridge code only.
    pub fn new(
        this_raw: jni::sys::jobject,
        method_name: &'static str,
        descriptor: &'static str,
    ) -> Self {
        Self {
            this_raw,
            method_name,
            descriptor,
        }
    }

    pub fn call<'local>(
        &self,
        env: &mut jni::JNIEnv<'local>,
        args: &[jni::objects::JValue<'_, '_>],
    ) -> jni::errors::Result<jni::objects::JValueOwned<'local>> {
        let this = unsafe { jni::objects::JObject::from_raw(self.this_raw) };
        env.call_method(this, self.method_name, self.descriptor, args)
    }
}
