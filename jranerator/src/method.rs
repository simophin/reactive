use std::collections::HashSet;

use convert_case::{Case, Casing};
use jni::signature::Primitive;
use quote::{format_ident, quote};
use syn::{parse_quote, Expr, Field, Ident, ItemFn, PathSegment, Type, TypePath, Visibility};

use crate::{
    class_like::{ClassLike, MethodDescription},
    sig::{JavaMethodDescription, JavaTypeDescription},
    utils::{java_primitive_array_to_rust, java_primitive_to_rust},
};

pub struct ArgInfo {
    pub name: Ident,
    pub ty: Type,
    pub is_java_primitive: bool,
}

pub struct JavaMethod {
    pub java_method_name: String,
    pub java_signature: String,
    pub java_method_desc: JavaMethodDescription,
    pub rust_method_id_cache_field: Ident,
    pub rust_method_name: Ident,
    pub rust_method_args: Vec<ArgInfo>,
    pub rust_method_return_type: Type,
    pub is_static: bool,
}

impl JavaMethod {
    pub fn from(java_class: &impl ClassLike) -> Vec<JavaMethod> {
        let mut rust_names: HashSet<String> = Default::default();

        java_class
            .get_public_methods()
            .into_iter()
            .map(
                |MethodDescription {
                     name: java_method_name,
                     signature,
                     is_static,
                     ..
                 }| {
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

                    let java_method_desc: JavaMethodDescription = signature
                        .parse()
                        .expect(&format!("parse signature '{signature}'"));

                    let rust_method_args = java_method_desc
                        .arguments
                        .iter()
                        .enumerate()
                        .map(|(index, java_type)| {
                            let name = format_ident!("arg{index}");
                            let ty: Type = match java_type {
                                JavaTypeDescription::Primitive(t) => {
                                    Type::Path(java_primitive_to_rust(&t))
                                }
                                JavaTypeDescription::String => {
                                    parse_quote! { &::jni::objects::JString<'_> }
                                }

                                JavaTypeDescription::Array(p) => match p.as_ref() {
                                    JavaTypeDescription::Primitive(t) => {
                                        Type::Path(java_primitive_array_to_rust(t))
                                    }
                                    JavaTypeDescription::Array(_)
                                    | JavaTypeDescription::Object(_)
                                    | JavaTypeDescription::String => {
                                        parse_quote! { &::jni::objects::JObjectArray<'_> }
                                    }
                                },

                                JavaTypeDescription::Object(_) => {
                                    parse_quote! { &::jni::objects::JObject<'_> }
                                }
                            };

                            let is_java_primitive =
                                matches!(java_type, JavaTypeDescription::Primitive(_));

                            ArgInfo {
                                name,
                                ty,
                                is_java_primitive,
                            }
                        })
                        .collect();

                    let rust_method_ret = match &java_method_desc.return_type {
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
                    };

                    JavaMethod {
                        java_method_name,
                        java_signature: signature,
                        rust_method_id_cache_field: format_ident!(
                            "r#{rust_name_candidate}_method_id_cache"
                        ),
                        rust_method_name: format_ident!("r#{rust_name_candidate}"),
                        rust_method_args,
                        rust_method_return_type: rust_method_ret,
                        java_method_desc,
                        is_static,
                    }
                },
            )
            .collect()
    }

    pub fn write_rust_struct_field(&self) -> Field {
        Field {
            vis: Visibility::Inherited,
            ident: Some(self.rust_method_id_cache_field.clone()),
            colon_token: Default::default(),
            ty: if self.is_static {
                parse_quote! { std::sync::OnceLock<Result<::jni::objects::JStaticMethodID, std::borrow::Cow<'static, str>>> }
            } else {
                parse_quote! { std::sync::OnceLock<Result<::jni::objects::JMethodID, std::borrow::Cow<'static, str>>> }
            },
            attrs: Default::default(),
            mutability: syn::FieldMutability::None,
        }
    }

    pub fn write_rust_fn(&self) -> ItemFn {
        let JavaMethod {
            java_method_name,
            java_signature,
            java_method_desc,
            rust_method_id_cache_field,
            rust_method_name,
            rust_method_args,
            rust_method_return_type: rust_method_ret,
            is_static,
        } = self;

        let jni_return_type: Expr = match &java_method_desc.return_type {
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
        };

        let method_call_assignments: Vec<Expr> = rust_method_args
            .iter()
            .map(
                |ArgInfo {
                     name,
                     is_java_primitive,
                     ..
                 }| {
                    if *is_java_primitive {
                        parse_quote! { ::jni::objects::JValueGen::<::jni::objects::JObject<'_>>::from(#name).as_jni() }
                    } else {
                        parse_quote! { ::jni::objects::JValueGen::Object(#name).as_jni() }
                    }
                },
            )
            .collect();

        let method_arg_declarations = rust_method_args
            .iter()
            .map(|ArgInfo { name, ty, .. }| {
                quote! { #name: #ty }
            })
            .collect::<Vec<_>>();

        if *is_static {
            parse_quote! {
                pub fn #rust_method_name<'local>(
                    &self,
                    env: &mut ::jni::JNIEnv<'local>,
                    #(#method_arg_declarations),*
                ) -> ::jni::errors::Result<#rust_method_ret> {
                    let method_id = match self.#rust_method_id_cache_field.get_or_init(|| {
                        env.get_static_method_id(
                            self.get_java_class(),
                            #java_method_name,
                            #java_signature,
                        ).map_err(|e| std::borrow::Cow::Owned(format!("Unable to find method '{}': {}", #java_method_name, e)))
                    }) {
                        Ok(v) => *v,
                        Err(_e) => {
                            return Err(::jni::errors::Error::MethodNotFound {
                                name: #java_method_name.to_string(),
                                sig: #java_signature.to_string(),
                            })
                        },
                    };

                    unsafe {
                        env.call_static_method_unchecked(
                            self.get_java_class(), method_id, #jni_return_type, &[#(#method_call_assignments),*]
                        )
                    }?.try_into()
                }
            }
        } else {
            parse_quote! {
                pub fn #rust_method_name<'local>(
                    &self,
                    env: &mut ::jni::JNIEnv<'local>,
                    obj: &::jni::objects::JObject<'_>,
                    #(#method_arg_declarations),*
                ) -> ::jni::errors::Result<#rust_method_ret> {
                    let method_id = match self.#rust_method_id_cache_field.get_or_init(|| {
                        env.get_method_id(
                            self.get_java_class(),
                            #java_method_name,
                            #java_signature,
                        ).map_err(|e| std::borrow::Cow::Owned(format!("Unable to find method '{}': {}", #java_method_name, e)))
                    }) {
                        Ok(v) => *v,
                        Err(_e) => return Err(::jni::errors::Error::MethodNotFound {
                            name: #java_method_name.to_string(),
                            sig: #java_signature.to_string(),
                        }),
                    };

                    unsafe {
                        env.call_method_unchecked(
                            obj, method_id, #jni_return_type, &[#(#method_call_assignments),*]
                        )
                    }?.try_into()
                }
            }
        }
    }
}
