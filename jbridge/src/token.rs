use jni::signature::Primitive;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};

use crate::{JavaArrayElementDescription, JavaTypeDescription};

impl<'a> ToTokens for JavaTypeDescription<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let new_tokens = match self {
            JavaTypeDescription::Primitive(p) => {
                let p_tokens = primitive_to_tokens(p);
                quote! {
                    ::jbridge::JavaTypeDescription::Primitive(#p_tokens)
                }
            }
            JavaTypeDescription::String => quote! {
                ::jbridge::JavaTypeDescription::String
            },
            JavaTypeDescription::Object { class_name } => quote! {
                ::jbridge::JavaTypeDescription::Object {
                    class_name: ::std::borrow::Cow::Borrowed(#class_name)
                }
            },
            JavaTypeDescription::Array(JavaArrayElementDescription::Primitive(p)) => {
                let p_tokens = primitive_to_tokens(p);
                quote! {
                    ::jbridge::JavaTypeDescription::Array(
                        ::jbridge::JavaArrayElementDescription::Primitive(#p_tokens)
                    )
                }
            }
            JavaTypeDescription::Array(JavaArrayElementDescription::ObjectLike { signature }) => {
                quote! {
                    ::jbridge::JavaTypeDescription::Array(
                        ::jbridge::JavaArrayElementDescription::ObjectLike {
                            signature: ::std::borrow::Cow::Borrowed(#signature)
                        }
                    )
                }
            }
        };

        tokens.extend(new_tokens);
    }
}

pub fn primitive_to_tokens(p: &Primitive) -> TokenStream {
    match p {
        Primitive::Boolean => quote! { ::jni::signature::Primitive::Boolean },
        Primitive::Byte => quote! { ::jni::signature::Primitive::Byte },
        Primitive::Char => quote! { ::jni::signature::Primitive::Char },
        Primitive::Short => quote! { ::jni::signature::Primitive::Short },
        Primitive::Int => quote! { ::jni::signature::Primitive::Int },
        Primitive::Long => quote! { ::jni::signature::Primitive::Long },
        Primitive::Float => quote! { ::jni::signature::Primitive::Float },
        Primitive::Double => quote! { ::jni::signature::Primitive::Double },
        Primitive::Void => quote! { ::jni::signature::Primitive::Void },
    }
}
