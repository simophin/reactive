use thiserror::Error;

#[derive(Error, Debug)]
pub enum InvocationError {
    #[error("Convert parameter `{name}` error: {err}")]
    ParameterConvertError {
        name: &'static str,
        err: Box<dyn std::error::Error>,
    },
    #[error("Convert return value error: {0}")]
    ReturnConvertError(Box<dyn std::error::Error>),
    #[error("JNI error: {0}")]
    JavaError(#[from] jni::errors::Error),
}

pub type Result<T> = std::result::Result<T, InvocationError>;
