use std::{borrow::Cow, collections::HashMap};

use convert_case::{Case, Casing};

use crate::class_like::{ClassLike, FieldDescription, MethodDescription};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldProperty {
    pub read_only: bool,
    pub desc: FieldDescription,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MethodProperty {
    GetterOnly(String, MethodDescription),
    SetterOnly(String, MethodDescription),
    ReadWrite {
        name: String,
        getter: MethodDescription,
        setter: MethodDescription,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JavaBeanProperty {
    Field(FieldProperty),
    Method(MethodProperty),
}

impl JavaBeanProperty {
    pub fn name(&self) -> Cow<str> {
        match self {
            JavaBeanProperty::Field(f) => Cow::Borrowed(&f.desc.name),
            JavaBeanProperty::Method(MethodProperty::GetterOnly(name, _))
            | JavaBeanProperty::Method(MethodProperty::SetterOnly(name, _))
            | JavaBeanProperty::Method(MethodProperty::ReadWrite { name, .. }) => {
                Cow::Borrowed(name.as_str())
            }
        }
    }
}

pub trait ClassLikePropsExt {
    fn get_properties(&self) -> Vec<JavaBeanProperty>;
}

impl<C: ClassLike> ClassLikePropsExt for C {
    fn get_properties(&self) -> Vec<JavaBeanProperty> {
        let mut properties = HashMap::new();

        // Put fields into properties
        for field in self
            .get_public_fields()
            .into_iter()
            .filter(|f| !f.is_static)
        {
            properties.insert(
                field.name.clone(),
                JavaBeanProperty::Field(FieldProperty {
                    read_only: field.is_final,
                    desc: field,
                }),
            );
        }

        // Inspect methods
        for method in self
            .get_public_methods()
            .into_iter()
            .filter(|m| !m.is_static)
        {
            if let Ok(prop) = MethodProperty::from(method) {
                match properties.get_mut(prop.name()) {
                    Some(JavaBeanProperty::Method(existing)) => {
                        existing.merge(prop);
                    }
                    _ => {
                        properties.insert(prop.name().to_owned(), JavaBeanProperty::Method(prop));
                    }
                }
            }
        }

        properties.into_values().collect()
    }
}

impl MethodProperty {
    pub fn name(&self) -> &str {
        match self {
            Self::GetterOnly(name, _)
            | Self::SetterOnly(name, _)
            | Self::ReadWrite { name, .. } => name,
        }
    }

    pub fn from(desc: MethodDescription) -> Result<Self, MethodDescription> {
        if desc.name.starts_with("get") {
            Ok(MethodProperty::GetterOnly(
                match Self::verified_property_name_in_method(&desc.name[3..]) {
                    Some(v) => v,
                    None => return Err(desc),
                },
                desc,
            ))
        } else if desc.name.starts_with("is") {
            Ok(MethodProperty::GetterOnly(
                match Self::verified_property_name_in_method(&desc.name[2..]) {
                    Some(v) => v,
                    None => return Err(desc),
                },
                desc,
            ))
        } else if desc.name.starts_with("set") {
            Ok(MethodProperty::SetterOnly(
                match Self::verified_property_name_in_method(&desc.name[3..]) {
                    Some(v) => v,
                    None => return Err(desc),
                },
                desc,
            ))
        } else {
            Err(desc)
        }
    }

    fn verified_property_name_in_method(name: &str) -> Option<String> {
        match name.chars().next() {
            Some(c) if c.is_ascii_uppercase() => Some(name.to_case(Case::Camel)),
            _ => None,
        }
    }

    pub fn merge(&mut self, o: MethodProperty) {
        match (&self, o) {
            (Self::GetterOnly(name, getter), Self::SetterOnly(_, setter))
            | (Self::GetterOnly(name, getter), Self::ReadWrite { setter, .. }) => {
                *self = Self::ReadWrite {
                    name: name.clone(),
                    getter: getter.clone(),
                    setter,
                };
            }

            (Self::SetterOnly(name, setter), Self::GetterOnly(_, getter))
            | (Self::SetterOnly(name, setter), Self::ReadWrite { getter, .. }) => {
                *self = Self::ReadWrite {
                    name: name.clone(),
                    getter,
                    setter: setter.clone(),
                };
            }

            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestData {
        methods: Vec<MethodDescription>,
        fields: Vec<FieldDescription>,
    }

    impl ClassLike for TestData {
        fn get_class_signature(&self) -> String {
            "Lcom/example/TestData;".to_owned()
        }

        fn get_public_methods(&self) -> Vec<MethodDescription> {
            self.methods.clone()
        }

        fn get_public_fields(&self) -> Vec<FieldDescription> {
            self.fields.clone()
        }
    }

    #[test]
    fn get_property_works() {
        let data = TestData {
            methods: vec![
                MethodDescription {
                    name: "getFoo".to_owned(),
                    signature: "()Ljava/lang/String;".to_owned(),
                    is_static: false,
                    argument_names: None,
                },
                MethodDescription {
                    name: "setFoo".to_owned(),
                    signature: "(Ljava/lang/String;)V".to_owned(),
                    is_static: false,
                    argument_names: None,
                },
                MethodDescription {
                    name: "getBar".to_owned(),
                    signature: "()Ljava/lang/String;".to_owned(),
                    is_static: false,
                    argument_names: None,
                },
            ],
            fields: vec![FieldDescription {
                name: "baz".to_owned(),
                signature: "Ljava/lang/String;".to_owned(),
                is_static: false,
                is_final: true,
            }],
        };

        let props = data.get_properties();
        assert_eq!(
            props,
            vec![
                JavaBeanProperty::Field(FieldProperty {
                    read_only: true,
                    desc: FieldDescription {
                        name: "baz".to_owned(),
                        signature: "Ljava/lang/String;".to_owned(),
                        is_static: false,
                        is_final: true,
                    },
                }),
                JavaBeanProperty::Method(MethodProperty::ReadWrite {
                    name: "foo".to_owned(),
                    getter: MethodDescription {
                        name: "getFoo".to_owned(),
                        signature: "()Ljava/lang/String;".to_owned(),
                        is_static: false,
                        argument_names: None,
                    },
                    setter: MethodDescription {
                        name: "setFoo".to_owned(),
                        signature: "(Ljava/lang/String;)V".to_owned(),
                        is_static: false,
                        argument_names: None,
                    },
                }),
                JavaBeanProperty::Method(MethodProperty::GetterOnly(
                    "bar".to_owned(),
                    MethodDescription {
                        name: "getBar".to_owned(),
                        signature: "()Ljava/lang/String;".to_owned(),
                        is_static: false,
                        argument_names: None,
                    },
                )),
            ]
        );
    }
}
