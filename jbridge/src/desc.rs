use std::{borrow::Cow, fmt::Display, str::FromStr};

use jni::signature::{Primitive, ReturnType};

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

impl<'a> From<JavaTypeDescription<'a>> for ReturnType {
    fn from(value: JavaTypeDescription<'a>) -> Self {
        match value {
            JavaTypeDescription::Primitive(p) => ReturnType::Primitive(p),
            JavaTypeDescription::Object { .. } | JavaTypeDescription::String => ReturnType::Object,
            JavaTypeDescription::Array(_) => ReturnType::Array,
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
    Boolean,
    Byte,
    Char,
    Double,
    Float,
    Int,
    Long,
    Short,
    ObjectLike { signature: Cow<'a, str> },
}

impl<'a> Display for JavaArrayElementDescription<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JavaArrayElementDescription::Boolean => write!(f, "Z"),
            JavaArrayElementDescription::Byte => write!(f, "B"),
            JavaArrayElementDescription::Char => write!(f, "C"),
            JavaArrayElementDescription::Double => write!(f, "D"),
            JavaArrayElementDescription::Float => write!(f, "F"),
            JavaArrayElementDescription::Int => write!(f, "I"),
            JavaArrayElementDescription::Long => write!(f, "J"),
            JavaArrayElementDescription::Short => write!(f, "S"),
            JavaArrayElementDescription::ObjectLike { signature } => {
                f.write_str(signature.as_ref())
            }
        }
    }
}

impl<'a> JavaArrayElementDescription<'a> {
    pub fn into_owned(self) -> JavaArrayElementDescription<'static> {
        match self {
            JavaArrayElementDescription::Boolean => JavaArrayElementDescription::Boolean,
            JavaArrayElementDescription::Byte => JavaArrayElementDescription::Byte,
            JavaArrayElementDescription::Char => JavaArrayElementDescription::Char,
            JavaArrayElementDescription::Double => JavaArrayElementDescription::Double,
            JavaArrayElementDescription::Float => JavaArrayElementDescription::Float,
            JavaArrayElementDescription::Int => JavaArrayElementDescription::Int,
            JavaArrayElementDescription::Long => JavaArrayElementDescription::Long,
            JavaArrayElementDescription::Short => JavaArrayElementDescription::Short,
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

impl FromStr for JavaTypeDescription<'static> {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (_, value) = parse_java_type(s).map_err(|_| "Invalid signature")?;
        Ok(value.into_owned())
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

impl<'a> JavaMethodDescription<'a> {
    pub fn into_owned(self) -> JavaMethodDescription<'static> {
        JavaMethodDescription {
            arguments: self
                .arguments
                .into_iter()
                .map(JavaTypeDescription::into_owned)
                .collect(),
            return_type: self.return_type.into_owned(),
        }
    }
}

impl<'a> TryFrom<&'a str> for JavaMethodDescription<'a> {
    type Error = nom::Err<nom::error::Error<&'a str>>;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let (_, value) = parse_java_method(value)?;
        Ok(value)
    }
}

impl FromStr for JavaMethodDescription<'static> {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (_, value) = parse_java_method(s).map_err(|_| "Invalid signature")?;
        Ok(value.into_owned())
    }
}
