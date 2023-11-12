use std::borrow::Cow;

use crate::ToJavaValue;

pub struct MethodSignatureBuilder {
    arguments: Vec<Cow<'static, str>>,
}

impl MethodSignatureBuilder {
    pub fn new() -> Self {
        Self {
            arguments: Vec::new(),
        }
    }

    pub fn add_argument<T: ToJavaValue>(self) -> Self {
        let Self { mut arguments } = self;
        arguments.push(T::java_signature());
        Self { arguments }
    }

    pub fn build<RetT: ToJavaValue>(self) -> String {
        let return_type = RetT::java_signature();
        format!("({}){}", self.arguments.join(""), return_type)
    }
}
