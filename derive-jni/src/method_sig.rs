use smallvec::SmallVec;

use crate::ToJavaValue;

pub struct MethodSignatureBuilder {
    arguments: SmallVec<[&'static str; 4]>,
}

impl MethodSignatureBuilder {
    pub fn new() -> Self {
        Self {
            arguments: Default::default(),
        }
    }

    pub fn add_argument<T: ToJavaValue>(self) -> Self {
        let Self { mut arguments } = self;
        arguments.push(T::SIGNATURE);
        Self { arguments }
    }

    pub fn build<RetT: ToJavaValue>(self) -> String {
        let return_type = RetT::SIGNATURE;
        format!("({}){}", self.arguments.join(""), return_type)
    }
}
