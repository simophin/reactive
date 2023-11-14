mod invocation_error;
mod method_sig;
pub mod value;

use jni::{objects::JObject, JNIEnv};

pub use invocation_error::InvocationError;
pub use invocation_error::Result as InvocationResult;
pub use method_sig::MethodSignatureBuilder;

pub use derive_jni_macro::java_class;

pub trait WithJavaObject {
    fn get_java_object(&self) -> Result<&JObject<'_>, jni::errors::Error>;
}

pub trait ToJavaValue {
    type JavaType<'a>;

    type ConvertError: std::error::Error;
    type BoxingError: std::error::Error;

    const SIGNATURE: &'static str;
    const BOXED_SIGNATURE: &'static str;

    fn into_java_value<'s>(
        &self,
        env: &mut JNIEnv<'s>,
    ) -> Result<Self::JavaType<'s>, Self::ConvertError>;

    fn into_java_value_boxed<'s>(
        &self,
        env: &mut JNIEnv<'s>,
    ) -> Result<JObject<'s>, Self::BoxingError>;
}

pub trait ToRustType<RustType: 'static> {
    type BoxedRustType: 'static;
    type Error: std::error::Error;
    type UnboxingError: std::error::Error;

    fn to_rust_type(&self, env: &mut JNIEnv<'_>) -> Result<RustType, Self::Error>;

    fn boxed_to_rust_type<'a>(
        env: &mut JNIEnv<'a>,
        obj: JObject<'a>,
    ) -> Result<Self::BoxedRustType, Self::UnboxingError>;
}
