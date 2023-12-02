use quote::format_ident;
use syn::{parse_quote, Field, File, ItemFn};

use crate::{class_like::ClassLike, field::JavaField, method::JavaMethod};

pub fn generate_binding(java_class: &impl ClassLike, name_override: Option<&str>) -> String {
    let methods = JavaMethod::from(java_class);
    let method_id_fields: Vec<Field> = methods
        .iter()
        .map(|m| m.write_rust_struct_field())
        .collect();
    let method_id_field_names: Vec<_> = method_id_fields.iter().map(|f| &f.ident).collect();
    let method_impl: Vec<ItemFn> = methods.iter().map(JavaMethod::write_rust_fn).collect();
    let class_signature = java_class.get_class_signature();

    let struct_name = match name_override {
        Some(name) => format_ident!("{}", name),
        None => format_ident!("{}", java_class.get_package_and_name().1.replace('$', "_")),
    };

    let fields = JavaField::from(java_class);

    let field_id_fields = fields.iter().map(|f| f.write_rust_field());
    let field_id_field_names = fields.iter().map(|f| f.write_rust_field().ident.unwrap());
    let field_impl = fields.iter().flat_map(|f| f.write_rust_methods());

    let content: File = parse_quote! {
        #[derive(Clone)]
        pub struct #struct_name {
            java_class: ::jni::objects::GlobalRef,
            #(#method_id_fields,)*
            #(#field_id_fields,)*
        }

        impl #struct_name {
            pub fn new<'local>(env: &mut ::jni::JNIEnv<'local>) -> ::jni::errors::Result<Self> {
                let java_class = env.find_class(#class_signature)?;
                let java_class = env.new_global_ref(java_class)?;
                Ok(Self {
                    java_class,
                    #(#method_id_field_names: Default::default(),)*
                    #(#field_id_field_names: Default::default(),)*
                })
            }

            pub fn get_java_class<'local>(&self) -> ::jni::objects::JClass<'local> {
                let raw = self.java_class.as_raw() as ::jni::sys::jclass;
                unsafe { ::jni::objects::JClass::from_raw(raw) }
            }

            #(#method_impl)*

            #(#field_impl)*
        }
    };

    prettyplease::unparse(&content)
}
