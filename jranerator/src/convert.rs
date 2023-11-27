use std::collections::HashSet;

use classy::{Attribute, ClassFile, Constant, ACC_PUBLIC, ACC_STATIC};
use convert_case::{Case, Casing};
use jni::signature::{JavaType, Primitive, TypeSignature};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{parse_quote, Field, FnArg, ItemFn, ReturnType, Type, TypePath, Visibility};

struct JavaMethod<'a> {
    java_method_name: &'a str,
    java_signature: &'a str,
    rust_method_id_cache_field: Ident,
    rust_method_name: Ident,
    rust_method_args: Vec<FnArg>,
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

pub fn convert_class(visibility: Visibility, struct_name: Ident, file: &ClassFile) -> TokenStream {
    let mut rust_names: HashSet<String> = Default::default();

    let methods: Vec<_> = file
        .method_info
        .iter()
        .filter(|&m| m.access_flags & ACC_PUBLIC != 0)
        .map(|method| {
            let java_method_name = file
                .get_constant_utf8(method.name_index)
                .expect("a method name");

            let is_static = method.access_flags & ACC_STATIC != 0;

            let signature = file
                .get_constant_utf8(method.descriptor_index)
                .expect("a signature");

            let initial_name_candidate = java_method_name.to_case(Case::Snake);
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

            let type_signature = TypeSignature::from_str(signature)
                .expect(&format!("parse signature '{signature}'"));

            let rust_method_args = type_signature
                .args
                .into_iter()
                .enumerate()
                .map(|(index, java_type)| {
                    let arg_name = format_ident!("arg{index}");
                    let t: TypePath = match java_type {
                        JavaType::Primitive(t) => java_primitive_to_rust(&t),

                        JavaType::Array(p) => match p.as_ref() {
                            JavaType::Primitive(t) => java_primitive_array_to_rust(t),
                            JavaType::Array(_) | JavaType::Object(_) | JavaType::Method(_) => {
                                parse_quote! { ::jni::objects::JObjectArray<'local> }
                            }
                        },

                        JavaType::Object(_) | JavaType::Method(_) => {
                            parse_quote! { ::jni::objects::JObjectArray<'local> }
                        }
                    };

                    parse_quote! { #arg_name: #t }
                })
                .collect();

            let rust_method_ret = match type_signature.ret {
                jni::signature::ReturnType::Primitive(p) => match p {
                    Primitive::Void => ReturnType::Default,
                    p => {
                        let t = java_primitive_to_rust(&p);
                        parse_quote! { -> #t }
                    }
                },

                jni::signature::ReturnType::Object | jni::signature::ReturnType::Array => {
                    parse_quote! { -> ::jni::objects::JObject<'local> }
                }
            };

            JavaMethod {
                java_method_name,
                java_signature: signature,
                rust_method_id_cache_field: format_ident!("{rust_name_candidate}_method_id"),
                rust_method_name: format_ident!("{rust_name_candidate}"),
                rust_method_args,
                rust_method_ret,
                is_static,
            }
        })
        .collect();

    let fields: Vec<Field> = methods
        .iter()
        .map(|m| Field {
            vis: Visibility::Inherited,
            ident: Some(m.rust_method_id_cache_field.clone()),
            colon_token: Default::default(),
            ty: parse_quote! { std::sync::OnceLock<::jni::sys::jmethodID> },
            attrs: Default::default(),
            mutability: syn::FieldMutability::None,
        })
        .collect();

    let method_impl: Vec<ItemFn> = methods.iter().map(generate_method_impl).collect();

    let Some(class_signature) = file.constant_pool.iter().find_map(|c| match c {
        Constant::ClassInfo { name_index } => file.get_constant_utf8(*name_index).ok(),
        _ => None,
    }) else {
        panic!("Class signature not found");
    };

    quote! {
        #visibility struct #struct_name {
            java_class: std::sync::OnceLock<::jni::errors::Result<::jni::objects::GlobalRef>>,
            #(#fields),*
        }

        impl #struct_name {
            fn get_class<'local>(&self, env: &mut ::jni::JNIEnv<'local>) ->
                ::jni::errors::Result<&::jni::JObject<'local>> {
                self.java_class.get_or_init(|| {
                    env.find_class(#class_signature)
                        .and_then(|c| env.new_global_ref(c))
                })
            }

            #(#method_impl)*
        }
    }
}

fn generate_method_impl(m: &JavaMethod<'_>) -> ItemFn {
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
        parse_quote! {}
    } else {
        parse_quote! {
            pub fn #rust_method_name<'local>(
                &self,
                env: &mut ::jni::JNIEnv<'local>,
                obj: ::jni::objects::JObject<'local>,
                #(#rust_method_args),*
            ) #rust_method_ret {
                let method_id = self.#rust_method_id_cache_field.get_or_init(|| {
                    env.get_method_id(
                        obj,
                        #java_method_name,
                        #java_signature,
                    )
                })?.clone();

                #(
                    let #rust_method_args = #rust_method_args.into_jvalue(env).expect("To convert argument");
                )*

                let ret = if #is_static {
                    env.call_static_method_unchecked(
                        obj,
                        method_id,
                        &[#(#rust_method_args),*],
                    )
                } else {
                    env.call_method_unchecked(
                        obj,
                        method_id,
                        &[#(#rust_method_args),*],
                    )
                };

                match ret {
                    Ok(v) => v,
                    Err(e) => panic!("Failed to call method: {:?}", e),
                }
            }
        }
    }
}
