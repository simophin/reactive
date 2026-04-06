pub mod class_def;
pub mod output;
mod writer;

pub use dexer_macros::dex_class;

pub use class_def::{AccessFlags, ClassDef, FieldDef, MethodCode, MethodEntry};
pub use output::{DexOutput, NativeMethod, NativeRegistrations};

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
        Self { this_raw, method_name, descriptor }
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
