use jni::signature::JavaType;

use crate::{JavaSingularTypeDescription, JavaTypeDescription};

impl Into<JavaType> for JavaTypeDescription {
    fn into(self) -> JavaType {
        match self {
            JavaTypeDescription::Single(_) => todo!(),
            JavaTypeDescription::Array(_) => todo!(),
        }
    }
}

impl Into<JavaType> for JavaSingularTypeDescription {
    fn into(self) -> JavaType {
        match self {
            JavaSingularTypeDescription::Primitive(p) => JavaType::Primitive(p),
            JavaSingularTypeDescription::String | JavaSingularTypeDescription::Object => JavaType::Object(()),
        }
    }
}
