use jbridge::{JavaArrayElementDescription, JavaTypeDescription};
use jni::signature::Primitive;
use syn::{parse_quote, Expr, Type};

pub trait JavaTypeDescriptionExt {
    fn to_tokens(&self) -> Expr;

    fn write_jni_type(&self) -> Type;
    fn write_jni_return_type(&self) -> Expr;
}

impl<'a> JavaTypeDescriptionExt for JavaTypeDescription<'a> {
    fn to_tokens(&self) -> Expr {
        match self {
            JavaTypeDescription::Primitive(p) => {
                let p_tokens = primitive_to_tokens(p);
                parse_quote! {
                    ::jbridge::JavaTypeDescription::Primitive(#p_tokens)
                }
            }
            JavaTypeDescription::String => parse_quote! {
                ::jbridge::JavaTypeDescription::String
            },
            JavaTypeDescription::Object { class_name } => parse_quote! {
                ::jbridge::JavaTypeDescription::Object {
                    class_name: ::std::borrow::Cow::Borrowed(#class_name)
                }
            },
            JavaTypeDescription::Array(JavaArrayElementDescription::Boolean) => {
                parse_quote! {
                    ::jbridge::JavaTypeDescription::Array(
                        ::jbridge::JavaArrayElementDescription::Boolean
                    )
                }
            }
            JavaTypeDescription::Array(JavaArrayElementDescription::Byte) => {
                parse_quote! {
                    ::jbridge::JavaTypeDescription::Array(
                        ::jbridge::JavaArrayElementDescription::Byte
                    )
                }
            }
            JavaTypeDescription::Array(JavaArrayElementDescription::Char) => {
                parse_quote! {
                    ::jbridge::JavaTypeDescription::Array(
                        ::jbridge::JavaArrayElementDescription::Char
                    )
                }
            }
            JavaTypeDescription::Array(JavaArrayElementDescription::Double) => {
                parse_quote! {
                    ::jbridge::JavaTypeDescription::Array(
                        ::jbridge::JavaArrayElementDescription::Double
                    )
                }
            }
            JavaTypeDescription::Array(JavaArrayElementDescription::Float) => {
                parse_quote! {
                    ::jbridge::JavaTypeDescription::Array(
                        ::jbridge::JavaArrayElementDescription::Float
                    )
                }
            }
            JavaTypeDescription::Array(JavaArrayElementDescription::Int) => {
                parse_quote! {
                    ::jbridge::JavaTypeDescription::Array(
                        ::jbridge::JavaArrayElementDescription::Int
                    )
                }
            }
            JavaTypeDescription::Array(JavaArrayElementDescription::Long) => {
                parse_quote! {
                    ::jbridge::JavaTypeDescription::Array(
                        ::jbridge::JavaArrayElementDescription::Long
                    )
                }
            }
            JavaTypeDescription::Array(JavaArrayElementDescription::Short) => {
                parse_quote! {
                    ::jbridge::JavaTypeDescription::Array(
                        ::jbridge::JavaArrayElementDescription::Short
                    )
                }
            }
            JavaTypeDescription::Array(JavaArrayElementDescription::ObjectLike { signature }) => {
                parse_quote! {
                    ::jbridge::JavaTypeDescription::Array(
                        ::jbridge::JavaArrayElementDescription::ObjectLike {
                            signature: ::std::borrow::Cow::Borrowed(#signature)
                        }
                    )
                }
            }
        }
    }

    fn write_jni_type(&self) -> Type {
        match self {
            JavaTypeDescription::Primitive(Primitive::Boolean) => {
                parse_quote! { ::jni::sys::jboolean }
            }
            JavaTypeDescription::Primitive(Primitive::Byte) => {
                parse_quote! { ::jni::sys::jbyte }
            }
            JavaTypeDescription::Primitive(Primitive::Char) => {
                parse_quote! { ::jni::sys::jchar }
            }
            JavaTypeDescription::Primitive(Primitive::Short) => {
                parse_quote! { ::jni::sys::jshort }
            }
            JavaTypeDescription::Primitive(Primitive::Int) => {
                parse_quote! { ::jni::sys::jint }
            }
            JavaTypeDescription::Primitive(Primitive::Long) => {
                parse_quote! { ::jni::sys::jlong }
            }
            JavaTypeDescription::Primitive(Primitive::Float) => {
                parse_quote! { ::jni::sys::jfloat }
            }
            JavaTypeDescription::Primitive(Primitive::Double) => {
                parse_quote! { ::jni::sys::jdouble }
            }
            JavaTypeDescription::Primitive(Primitive::Void) => {
                parse_quote! { () }
            }

            JavaTypeDescription::String => parse_quote! { ::jni::objects::JString<'_> },
            JavaTypeDescription::Object { .. } => parse_quote! { ::jni::objects::JObject<'_> },
            JavaTypeDescription::Array(JavaArrayElementDescription::Boolean) => {
                parse_quote! { ::jni::objects::JBooleanArray<'_> }
            }
            JavaTypeDescription::Array(JavaArrayElementDescription::Byte) => {
                parse_quote! { ::jni::objects::JByteArray<'_> }
            }
            JavaTypeDescription::Array(JavaArrayElementDescription::Char) => {
                parse_quote! { ::jni::objects::JCharArray<'_> }
            }
            JavaTypeDescription::Array(JavaArrayElementDescription::Short) => {
                parse_quote! { ::jni::objects::JShortArray<'_> }
            }
            JavaTypeDescription::Array(JavaArrayElementDescription::Int) => {
                parse_quote! { ::jni::objects::JIntArray<'_> }
            }
            JavaTypeDescription::Array(JavaArrayElementDescription::Long) => {
                parse_quote! { ::jni::objects::JLongArray<'_> }
            }
            JavaTypeDescription::Array(JavaArrayElementDescription::Short) => {
                parse_quote! { ::jni::objects::JFloatArray<'_> }
            }
            JavaTypeDescription::Array(JavaArrayElementDescription::Double) => {
                parse_quote! { ::jni::objects::JDoubleArray<'_> }
            }
            JavaTypeDescription::Array(JavaArrayElementDescription::Float) => {
                parse_quote! { ::jni::objects::JFloatArray<'_> }
            }
            JavaTypeDescription::Array(JavaArrayElementDescription::ObjectLike { .. }) => {
                parse_quote! { ::jni::objects::JObjectArray<'_> }
            }
        }
    }

    fn write_jni_return_type(&self) -> Expr {
        match self {
            JavaTypeDescription::Primitive(p) => {
                let p_token = primitive_to_tokens(p);
                parse_quote! {
                   ::jni::signature::ReturnType::Primitive(#p_token)
                }
            }

            JavaTypeDescription::Object { .. } | JavaTypeDescription::String => parse_quote! {
                ::jni::signature::ReturnType::Object
            },

            JavaTypeDescription::Array(_) => parse_quote! {
                ::jni::signature::ReturnType::Array
            },
        }
    }
}

pub fn primitive_to_tokens(p: &Primitive) -> Type {
    match p {
        Primitive::Boolean => parse_quote! { ::jni::signature::Primitive::Boolean },
        Primitive::Byte => parse_quote! { ::jni::signature::Primitive::Byte },
        Primitive::Char => parse_quote! { ::jni::signature::Primitive::Char },
        Primitive::Short => parse_quote! { ::jni::signature::Primitive::Short },
        Primitive::Int => parse_quote! { ::jni::signature::Primitive::Int },
        Primitive::Long => parse_quote! { ::jni::signature::Primitive::Long },
        Primitive::Float => parse_quote! { ::jni::signature::Primitive::Float },
        Primitive::Double => parse_quote! { ::jni::signature::Primitive::Double },
        Primitive::Void => parse_quote! { ::jni::signature::Primitive::Void },
    }
}
