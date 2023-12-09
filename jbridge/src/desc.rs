use std::{borrow::Cow, fmt::Display};

use jni::signature::Primitive;

use crate::parse::{parse_java_method, parse_java_type};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JavaTypeDescription<'a> {
    Primitive(Primitive),
    String,
    Object { class_name: Cow<'a, str> },
    Array(JavaArrayElementDescription<'a>),
}

impl<'a> Display for JavaTypeDescription<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JavaTypeDescription::Primitive(p) => p.fmt(f),
            JavaTypeDescription::String => write!(f, "Ljava/lang/String;"),
            JavaTypeDescription::Object { class_name } => write!(f, "L{class_name};"),
            JavaTypeDescription::Array(element) => write!(f, "[{}", element),
        }
    }
}

impl<'a> JavaTypeDescription<'a> {
    pub fn into_owned(self) -> JavaTypeDescription<'static> {
        match self {
            JavaTypeDescription::Primitive(p) => JavaTypeDescription::Primitive(p),
            JavaTypeDescription::String => JavaTypeDescription::String,
            JavaTypeDescription::Object { class_name } => JavaTypeDescription::Object {
                class_name: Cow::Owned(class_name.into_owned()),
            },
            JavaTypeDescription::Array(element) => JavaTypeDescription::Array(element.into_owned()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JavaArrayElementDescription<'a> {
    Primitive(Primitive),
    ObjectLike { signature: Cow<'a, str> },
}

impl<'a> Display for JavaArrayElementDescription<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JavaArrayElementDescription::Primitive(p) => p.fmt(f),
            JavaArrayElementDescription::ObjectLike { signature } => {
                f.write_str(signature.as_ref())
            }
        }
    }
}

impl<'a> JavaArrayElementDescription<'a> {
    pub fn into_owned(self) -> JavaArrayElementDescription<'static> {
        match self {
            JavaArrayElementDescription::Primitive(p) => JavaArrayElementDescription::Primitive(p),
            JavaArrayElementDescription::ObjectLike { signature } => {
                JavaArrayElementDescription::ObjectLike {
                    signature: Cow::Owned(signature.into_owned()),
                }
            }
        }
    }
}

impl<'a> TryFrom<&'a str> for JavaTypeDescription<'a> {
    type Error = nom::Err<nom::error::Error<&'a str>>;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let (_, value) = parse_java_type(value)?;
        Ok(value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JavaMethodDescription<'a> {
    pub arguments: Vec<JavaTypeDescription<'a>>,
    pub return_type: JavaTypeDescription<'a>,
}

impl<'a> Display for JavaMethodDescription<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("(")?;
        for arg in &self.arguments {
            arg.fmt(f)?;
        }
        f.write_str(")")?;
        self.return_type.fmt(f)
    }
}

impl<'a> TryFrom<&'a str> for JavaMethodDescription<'a> {
    type Error = nom::Err<nom::error::Error<&'a str>>;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let (_, value) = parse_java_method(value)?;
        Ok(value)
    }
}
