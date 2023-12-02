use jni::signature::Primitive;
use syn::{parse_quote, Expr, PathSegment, Type};

use crate::{
    desc::JavaTypeDescription,
    utils::{java_primitive_array_to_rust, java_primitive_to_rust},
};

impl JavaTypeDescription {
    pub fn write_jni_return_type(&self) -> Expr {
        match self {
            JavaTypeDescription::Primitive(p) => {
                let path: PathSegment = match p {
                    Primitive::Boolean => parse_quote! { Boolean },
                    Primitive::Byte => parse_quote! { Byte },
                    Primitive::Char => parse_quote! { Char },
                    Primitive::Double => parse_quote! { Double },
                    Primitive::Float => parse_quote! { Float },
                    Primitive::Int => parse_quote! { Int },
                    Primitive::Long => parse_quote! { Long },
                    Primitive::Short => parse_quote! { Short },
                    Primitive::Void => parse_quote! { Void },
                };

                parse_quote! { ::jni::signature::ReturnType::Primitive(::jni::signature::Primitive::#path) }
            }

            JavaTypeDescription::Array(_) => parse_quote! { ::jni::signature::ReturnType::Array },
            _ => parse_quote! { ::jni::signature::ReturnType::Object },
        }
    }

    pub fn write_jni_type(&self) -> Type {
        match self {
            JavaTypeDescription::Primitive(p) => match p {
                Primitive::Void => parse_quote! { () },
                p => Type::Path(java_primitive_to_rust(&p)),
            },

            JavaTypeDescription::String => {
                parse_quote! { ::jni::objects::JString<'local> }
            }
            JavaTypeDescription::Object(_) => {
                parse_quote! { ::jni::objects::JObject<'local> }
            }
            JavaTypeDescription::Array(p) => match p.as_ref() {
                JavaTypeDescription::Primitive(t) => {
                    let t = java_primitive_array_to_rust(t);
                    parse_quote! { #t }
                }

                JavaTypeDescription::Array(_)
                | JavaTypeDescription::Object(_)
                | JavaTypeDescription::String => {
                    parse_quote! { ::jni::objects::JObjectArray<'local> }
                }
            },
        }
    }

    pub fn write_jni_java_type(&self) -> Type {
        todo!()
    }
}
