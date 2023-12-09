use proc_macro2::TokenStream;
use quote::quote;

use crate::{token::primitive_to_tokens, JavaTypeDescription};

impl<'a> JavaTypeDescription<'a> {
    pub fn as_jni_return_type_token(&self) -> TokenStream {
        match self {
            JavaTypeDescription::Primitive(p) => {
                let p_token = primitive_to_tokens(p);
                quote! {
                   ::jni::signature::ReturnType::Primitive(#p_token)
                }
            }

            JavaTypeDescription::Object { .. } | JavaTypeDescription::String => quote! {
                ::jni::signature::ReturnType::Object
            },

            JavaTypeDescription::Array(_) => quote! {
                ::jni::signature::ReturnType::Array
            },
        }
    }
}
