/// A compiled DEX class ready to load and register.
pub struct DexOutput {
    bytes: Vec<u8>,
    registrations: NativeRegistrations,
}

impl DexOutput {
    pub(crate) fn new(bytes: Vec<u8>, registrations: NativeRegistrations) -> Self {
        Self { bytes, registrations }
    }

    /// Raw DEX bytes — pass to `DexClassLoader`.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn registrations(&self) -> &NativeRegistrations {
        &self.registrations
    }
}

pub struct NativeRegistrations {
    pub class_name: String,
    pub methods: Vec<NativeMethod>,
}

pub struct NativeMethod {
    pub name: String,
    pub descriptor: String,
    pub fn_ptr: *mut std::ffi::c_void,
}

// SAFETY: fn pointers are never called here; caller is responsible for
// only invoking them on the correct thread after JNI class is loaded.
unsafe impl Send for NativeMethod {}
unsafe impl Sync for NativeMethod {}

impl NativeRegistrations {
    /// Calls `RegisterNatives` for every entry.
    pub fn register(
        &self,
        env: &mut jni::JNIEnv<'_>,
        class: &jni::objects::JClass<'_>,
    ) -> jni::errors::Result<()> {
        let methods: Vec<jni::NativeMethod> = self
            .methods
            .iter()
            .map(|m| jni::NativeMethod {
                name: m.name.clone().into(),
                sig: m.descriptor.clone().into(),
                fn_ptr: m.fn_ptr,
            })
            .collect();
        env.register_native_methods(class, &methods)
    }
}
