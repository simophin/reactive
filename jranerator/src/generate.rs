use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Field, ItemFn};

use crate::{class_like::ClassLike, method::JavaMethod};

pub fn generate_binding(java_class: &impl ClassLike) -> (Vec<String>, TokenStream) {
    let methods = JavaMethod::from(java_class);
    let method_id_fields: Vec<Field> = methods
        .iter()
        .map(|m| m.write_rust_struct_field())
        .collect();
    let method_id_field_names: Vec<_> = method_id_fields.iter().map(|f| &f.ident).collect();
    let method_impl: Vec<ItemFn> = methods.iter().map(JavaMethod::write_rust_fn).collect();
    let class_signature = java_class.get_class_signature();

    let modules = class_signature
        .split('/')
        .map(|s| s.to_case(Case::Snake))
        .collect::<Vec<_>>();

    let class_name = modules.last().expect("To have a class_name");
    let struct_name = format_ident!("{}", class_name);

    let content = quote! {
        pub struct #struct_name {
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

    (modules, content)
}
