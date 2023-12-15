use jni::errors::{Error, JniError};

pub trait ErrorExt {
    fn cloned(&self) -> Error;
}

impl ErrorExt for Error {
    fn cloned(&self) -> Error {
        match self {
            Error::WrongJValueType(a, b) => Error::WrongJValueType(*a, *b),
            Error::InvalidCtorReturn => Error::InvalidCtorReturn,
            Error::InvalidArgList(a) => Error::InvalidArgList(a.clone()),
            Error::MethodNotFound { name, sig } => todo!(),
            Error::FieldNotFound { name, sig } => todo!(),
            Error::JavaException => Error::JavaException,
            Error::JNIEnvMethodNotFound(a) => Error::JNIEnvMethodNotFound(a),
            Error::NullPtr(a) => Error::NullPtr(a),
            Error::NullDeref(a) => Error::NullDeref(a),
            Error::TryLock => Error::TryLock,
            Error::JavaVMMethodNotFound(a) => Error::JavaVMMethodNotFound(a),
            Error::FieldAlreadySet(a) => Error::FieldAlreadySet(a.clone()),
            Error::ThrowFailed(a) => Error::ThrowFailed(*a),
            Error::ParseFailed(_, _) => todo!(),
            Error::JniCall(JniError::AlreadyCreated) => Error::JniCall(JniError::AlreadyCreated),
            Error::JniCall(JniError::InvalidArguments) => {
                Error::JniCall(JniError::InvalidArguments)
            }
            Error::JniCall(JniError::NoMemory) => Error::JniCall(JniError::NoMemory),
            Error::JniCall(JniError::Other(a)) => Error::JniCall(JniError::Other(*a)),
            Error::JniCall(JniError::ThreadDetached) => Error::JniCall(JniError::ThreadDetached),
            Error::JniCall(JniError::Unknown) => Error::JniCall(JniError::Unknown),
            Error::JniCall(JniError::WrongVersion) => Error::JniCall(JniError::WrongVersion),
        }
    }
}
