use proc_macro2::Ident;
use quote::format_ident;
use syn::{parse_quote, Field, ItemFn};

use crate::{class_like::ClassLike, desc::JavaTypeDescription, utils::java_name_to_rust_name};

pub struct JavaField {
    desc: JavaTypeDescription,
    sig: String,
    is_static: bool,
    is_final: bool,
    java_name: String,
    rust_field_id_cache: Ident,
    rust_field_id_cache_access_func: Ident,
    rust_method_name: Ident,
}

impl JavaField {
    pub fn from(java_class: &impl ClassLike) -> Vec<JavaField> {
        java_class
            .get_public_fields()
            .iter()
            .map(|f| {
                let rust_name = java_name_to_rust_name(&f.name);
                JavaField {
                    desc: f.signature.parse().expect("A valid Java field signature"),
                    rust_method_name: format_ident!("r#{}", rust_name),
                    rust_field_id_cache: format_ident!("r#{}_id_cache", rust_name),
                    rust_field_id_cache_access_func: format_ident!("r#get_{}_field_id", rust_name),
                    sig: f.signature.clone(),
                    java_name: f.name.clone(),
                    is_static: f.is_static,
                    is_final: f.is_final,
                }
            })
            .collect()
    }

    pub fn write_rust_field(&self) -> Field {
        let Self {
            rust_field_id_cache,
            is_static,
            ..
        } = self;

        Field {
            attrs: Default::default(),
            vis: syn::Visibility::Inherited,
            mutability: syn::FieldMutability::None,
            ident: Some(format_ident!("r#{}", rust_field_id_cache)),
            colon_token: Default::default(),
            ty: if *is_static {
                parse_quote! { std::sync::OnceLock<Result<::jni::objects::JStaticFieldID, std::borrow::Cow<'static, str>>> }
            } else {
                parse_quote! { std::sync::OnceLock<Result<::jni::objects::JFieldID, std::borrow::Cow<'static, str>>> }
            },
        }
    }

    pub fn write_rust_methods(&self) -> Vec<ItemFn> {
        let Self {
            desc,
            is_static,
            rust_field_id_cache,
            rust_field_id_cache_access_func,
            rust_method_name,
            is_final,
            sig,
            java_name,
        } = self;

        let mut methods = Vec::new();
        let rust_type = desc.write_jni_type();
        let java_type = desc.write_jni_java_type();
        let getter_name = format_ident!("get_field_{}", rust_method_name);
        let setter_name = format_ident!("set_field_{}", rust_method_name);
        let return_value_conversion = desc.write_value_conversion(&format_ident!("ret"));
        let rust_setter_arg = match desc {
            JavaTypeDescription::Primitive(_) => rust_type.clone(),
            _ => parse_quote! { &#rust_type },
        };

        if *is_static {
            methods.push(parse_quote! {
                fn #rust_field_id_cache_access_func<'local>(&self, env: &mut ::jni::JNIEnv<'local>) -> ::jni::errors::Result<::jni::objects::JStaticFieldID> {
                    let field_id = self.#rust_field_id_cache.get_or_init(|| {
                        env.get_static_field_id(self.get_java_class(), #java_name, #sig)
                            .map_err(|e| std::borrow::Cow::Owned(format!("Unable to find field '{}': {}", #java_name, e)))
                    });

                    match field_id {
                        Ok(id) => Ok(*id),
                        Err(_) => {
                            Err(::jni::errors::Error::FieldNotFound {
                                name: #java_name.to_string(),
                                sig: #sig.to_string(),
                            })
                        }
                    }
                }
            });

            methods.push(parse_quote! {
                pub fn #getter_name<'local>(&self, env: &mut ::jni::JNIEnv<'local>) -> ::jni::errors::Result<#rust_type> {
                    let field_id = self.#rust_field_id_cache_access_func(env)?;
                    let ret = unsafe {
                        env.get_static_field_unchecked(self.get_java_class(), field_id, #java_type)
                    }?;

                    #return_value_conversion
                }
            });

            if *is_final {
                methods.push(parse_quote! {
                    pub fn #setter_name<'local>(&self, env: &mut ::jni::JNIEnv<'local>, value: #rust_setter_arg) -> ::jni::errors::Result<()> {
                        let field_id = self.#rust_field_id_cache_access_func(env)?;
                        env.set_static_field(
                            self.get_java_class(), 
                            field_id,
                            value.into()
                        )
                    }
                });
            }
        } else {
            methods.push(parse_quote! {
                fn #rust_field_id_cache_access_func<'local>(&self, env: &mut ::jni::JNIEnv<'local>) -> ::jni::errors::Result<::jni::objects::JFieldID> {
                    let field_id = self.#rust_field_id_cache.get_or_init(|| {
                        env.get_field_id(self.get_java_class(), #java_name, #sig)
                            .map_err(|e| std::borrow::Cow::Owned(format!("Unable to find field '{}': {}", #java_name, e)))
                    });

                    match field_id {
                        Ok(id) => Ok(*id),
                        Err(_) => {
                            Err(::jni::errors::Error::FieldNotFound {
                                name: #java_name.to_string(),
                                sig: #sig.to_string(),
                            })
                        }
                    }
                }
            });

            methods.push(parse_quote! {
                pub fn #getter_name<'local>(&self, env: &mut ::jni::JNIEnv<'local>, obj: &::jni::objects::JObject<'local>) -> ::jni::errors::Result<#rust_type> {
                    let field_id = self.#rust_field_id_cache_access_func(env)?;
                    let ret = unsafe {
                        env.get_field_unchecked(obj, field_id, #java_type)
                    }?;

                    #return_value_conversion
                }
            });

            if *is_final {
                methods.push(parse_quote! {
                    pub fn #setter_name<'local>(&self, env: &mut ::jni::JNIEnv<'local>, obj: &::jni::objects::JObject<'local>, value: #rust_setter_arg) -> ::jni::errors::Result<()> {
                        let field_id = self.#rust_field_id_cache_access_func(env)?;
                        env.set_field(
                            obj, 
                            field_id,
                            value.into()
                        )
                    }
                });
            }
        }

        methods
    }
}
