use convert_case::{Case, Casing};
use jni::signature::Primitive;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{parse_quote, Expr, Field, ItemFn, PathSegment, Type, TypePath, Visibility};

use crate::{
    class_like::{ClassLike, MethodDescription},
    method::JavaMethod,
    sig::{JavaMethodDescription, JavaTypeDescription},
};

pub fn convert_class(
    visibility: Visibility,
    struct_name: Ident,
    java_class: &impl ClassLike,
) -> TokenStream {
    let methods = JavaMethod::from(java_class);
    let method_id_fields: Vec<Field> = methods
        .iter()
        .map(|m| m.write_rust_struct_field())
        .collect();
    let method_id_field_names: Vec<_> = method_id_fields.iter().map(|f| &f.ident).collect();
    let method_impl: Vec<ItemFn> = methods.iter().map(JavaMethod::write_rust_fn).collect();
    let class_signature = java_class.get_class_signature();

    let content = quote! {
        #visibility struct #struct_name {
            java_class: ::jni::objects::GlobalRef,
            #(#method_id_fields),*
        }

        impl #struct_name {
            pub fn new<'local>(env: &mut ::jni::JNIEnv<'local>) -> ::jni::errors::Result<Self> {
                let java_class = env.find_class(#class_signature)?;
                let java_class = env.new_global_ref(java_class)?;
                Ok(Self {
                    java_class,
                    #(#method_id_field_names: Default::default()),*
                })
            }

            pub fn get_java_class<'local>(&self) -> ::jni::objects::JClass<'local> {
                let raw = self.java_class.as_raw() as ::jni::sys::jclass;
                unsafe { ::jni::objects::JClass::from_raw(raw) }
            }

            #(#method_impl)*
        }
    };

    class_signature
        .split('/')
        .rev()
        .skip(1)
        .map(|s| s.to_case(Case::Snake))
        .fold(content, |acc, mod_name| {
            let mod_name = format_ident!("{}", mod_name);
            parse_quote! {
                #visibility mod #mod_name {
                    #acc
                }
            }
        })
}
