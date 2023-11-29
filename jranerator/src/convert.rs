use std::collections::HashSet;

use convert_case::{Case, Casing};
use jni::signature::Primitive;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{
    parse_quote, Field, ItemFn, ItemMod, Pat, PatIdent, PatType, Type, TypePath, Visibility,
};

use crate::{
    class_like::{ClassLike, MethodDescription},
    sig::{JavaMethodDescription, JavaTypeDescription},
};

struct JavaMethod {
    java_method_name: String,
    java_signature: String,
    rust_method_id_cache_field: Ident,
    rust_method_name: Ident,
    rust_method_args: Vec<PatType>,
    rust_method_ret: Type,
    is_static: bool,
}

fn java_primitive_to_rust(primitive: &Primitive) -> TypePath {
    let path: TypePath = match primitive {
        Primitive::Boolean => parse_quote! { jboolean },
        Primitive::Byte => parse_quote! { jbyte },
        Primitive::Char => parse_quote! { jchar },
        Primitive::Double => parse_quote! { jdouble },
        Primitive::Float => parse_quote! { jfloat },
        Primitive::Int => parse_quote! { jint },
        Primitive::Long => parse_quote! { jlong },
        Primitive::Short => parse_quote! { jshort },
        Primitive::Void => parse_quote! { jvoid },
    };

    parse_quote! { ::jni::sys::#path }
}

fn java_primitive_array_to_rust(primitive: &Primitive) -> TypePath {
    let path: TypePath = match primitive {
        Primitive::Boolean => parse_quote! { JBooleanArray<'local> },
        Primitive::Byte => parse_quote! { JByteArray<'local> },
        Primitive::Char => parse_quote! { JCharArray<'local> },
        Primitive::Double => parse_quote! { JDoubleArray<'local> },
        Primitive::Float => parse_quote! { JFloatArray<'local> },
        Primitive::Int => parse_quote! { JIntArray<'local> },
        Primitive::Long => parse_quote! { JLongArray<'local> },
        Primitive::Short => parse_quote! { JShortArray<'local> },
        Primitive::Void => panic!("Void arrays are not supported"),
    };

    parse_quote! { ::jni::objects::#path }
}

pub fn convert_class(
    visibility: Visibility,
    struct_name: Ident,
    java_class: &impl ClassLike,
) -> TokenStream {
    let mut rust_names: HashSet<String> = Default::default();

    let methods: Vec<_> = java_class
        .get_public_methods()
        .into_iter()
        .map(|MethodDescription { name: java_method_name, signature, is_static, .. }| {
            let (initial_name_candidate, is_static) = match java_method_name.as_str() {
                "<init>" => ("new_instance".to_string(), true),
                n => (n.to_case(Case::Snake), is_static),
            };

            let rust_name_candidate = (0..std::usize::MAX)
                .map(|index| {
                    if index == 0 {
                        initial_name_candidate.clone()
                    } else {
                        format!("{initial_name_candidate}_{index}")
                    }
                })
                .filter(|name| !rust_names.contains(name))
                .next()
                .expect("Unable to find a suitable rust name");

            rust_names.insert(rust_name_candidate.clone());

            let JavaMethodDescription { arguments, return_type } = signature
                .parse()
                .expect(&format!("parse signature '{signature}'"));

            let rust_method_args = arguments
                .into_iter()
                .enumerate()
                .map(|(index, java_type)| {
                    let arg_name = format_ident!("arg{index}");
                    let t: Type = match java_type {
                        JavaTypeDescription::Primitive(t) => Type::Path(java_primitive_to_rust(&t)),
                        JavaTypeDescription::String => parse_quote! { &::jni::objects::JString<'_> },

                        JavaTypeDescription::Array(p) => match p.as_ref() {
                            JavaTypeDescription::Primitive(t) => Type::Path(java_primitive_array_to_rust(t)),
                            JavaTypeDescription::Array(_) | JavaTypeDescription::Object(_) | JavaTypeDescription::String => {
                                parse_quote! { &::jni::objects::JObjectArray<'_> }
                            }
                        },

                        JavaTypeDescription::Object(_) => parse_quote! { &::jni::objects::JObject<'_> },
                    };

                    PatType {
                        pat: Box::new(Pat::Ident(PatIdent {
                            attrs: Default::default(),
                            by_ref: None,
                            mutability: None,
                            ident: arg_name,
                            subpat: None,
                        })),
                        attrs: Default::default(),
                        colon_token: Default::default(),
                        ty: Box::new(t),
                    }
                })
                .collect();

            let rust_method_ret = match return_type {
                JavaTypeDescription::Primitive(p) => match p {
                    Primitive::Void => parse_quote! { () },
                    p => Type::Path(java_primitive_to_rust(&p)),
                },

                JavaTypeDescription::String => parse_quote! { jni::objects::AutoLocal<'local, ::jni::objects::JString<'local>> },
                JavaTypeDescription::Object(_) => parse_quote! { jni::objects::AutoLocal<'local, ::jni::objects::JObject<'local>> },
                JavaTypeDescription::Array(p) => match p.as_ref() {
                    JavaTypeDescription::Primitive(t) => {
                        let t = java_primitive_array_to_rust(t);
                        parse_quote! { jni::objects::AutoLocal<'local, #t> }
                    }

                    JavaTypeDescription::Array(_) | JavaTypeDescription::Object(_) | JavaTypeDescription::String => {
                        parse_quote! { jni::objects::AutoLocal<'local, ::jni::objects::JObjectArray<'local>> }
                    }
                },
            };

            JavaMethod {
                java_method_name,
                java_signature: signature,
                rust_method_id_cache_field: format_ident!("r#{rust_name_candidate}_method_id_cache"),
                rust_method_name: format_ident!("r#{rust_name_candidate}"),
                rust_method_args,
                rust_method_ret,
                is_static,
            }
        })
        .collect();

    let method_id_fields: Vec<Field> = methods
        .iter()
        .map(|m| Field {
            vis: Visibility::Inherited,
            ident: Some(m.rust_method_id_cache_field.clone()),
            colon_token: Default::default(),
            ty: if m.is_static {
                parse_quote! { std::cell::OnceCell<::jni::errors::Result<::jni::objects::JStaticMethodID>> }
            } else {
                parse_quote! { std::cell::OnceCell<::jni::errors::Result<::jni::objects::JMethodID>> }
            },
            attrs: Default::default(),
            mutability: syn::FieldMutability::None,
        })
        .collect();

    let method_id_field_names: Vec<_> = method_id_fields.iter().map(|f| &f.ident).collect();
    let method_impl: Vec<ItemFn> = methods.iter().map(generate_method_impl).collect();
    let class_signature = java_class.get_class_signature();

    let content = quote! {
        #visibility struct #struct_name {
            java_class: ::jni::objects::GlobalRef,
            #(#method_id_fields),*
        }

        impl #struct_name {
            pub fn new<'local>(env: &mut ::jni::JNIEnv<'local>) -> ::jni::errors::Result<Self> {
                let java_class = env.new_global_ref(env.find_class(#class_signature)?)?;
                Ok(Self {
                    java_class,
                    #(#method_id_field_names: Default::default()),*
                })
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

fn generate_method_impl(m: &JavaMethod) -> ItemFn {
    let JavaMethod {
        java_method_name,
        java_signature,
        rust_method_id_cache_field,
        rust_method_name,
        rust_method_args,
        rust_method_ret,
        is_static,
    } = m;

    if *is_static {
        parse_quote! {
            pub fn #rust_method_name<'local>(
                &self,
                env: &mut ::jni::JNIEnv<'local>,
                obj: &::jni::objects::JObject<'_>,
                #(#rust_method_args),*
            ) -> ::jni::errors::Result<#rust_method_ret> {
                let method_id = match self.#rust_method_id_cache_field.get_or_init(|| {
                    env.get_static_method_id(
                        self.java_class.as_ref(),
                        #java_method_name,
                        #java_signature,
                    )
                }) {
                    Ok(v) => *v,
                    Err(e) => return Err(e.clone()),
                };

                todo!()
            }
        }
    } else {
        parse_quote! {
            pub fn #rust_method_name<'local>(
                &self,
                env: &mut ::jni::JNIEnv<'local>,
                obj: &::jni::objects::JObject<'_>,
                #(#rust_method_args),*
            ) -> ::jni::errors::Result<#rust_method_ret> {
                let method_id = match self.#rust_method_id_cache_field.get_or_init(|| {
                    env.get_method_id(
                        self.java_class.as_ref(),
                        #java_method_name,
                        #java_signature,
                    )
                }) {
                    Ok(v) => *v,
                    Err(e) => return Err(e.clone()),
                };

                todo!()
            }
        }
    }
}
