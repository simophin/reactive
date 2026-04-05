use proc_macro2::{Span, TokenStream};
use quote::quote;

use super::ast::{JavaBinding, JavaType};

pub fn expand(binding: JavaBinding) -> TokenStream {
    let JavaBinding {
        class,
        fields,
        methods,
    } = binding;
    let class_ident = class.last().clone();
    let class_name = class.jni_name();

    let mut output = TokenStream::new();
    let mut emitted_fields = std::collections::BTreeSet::new();
    let mut emitted_methods = std::collections::BTreeSet::new();

    output.extend(quote! {
        pub struct #class_ident;

        impl ::android::JavaClassDescriptor for #class_ident {
            const FQ_NAME: &'static str = #class_name;
        }
    });

    for field in fields {
        let field_name = field.name.clone();
        if emitted_fields.insert(field_name.to_string()) {
            output.extend(quote! {
                #[allow(non_camel_case_types)]
                pub struct #field_name;
            });
        }

        let signature = match java_type_signature(&field.ty, false) {
            Ok(sig) => sig,
            Err(err) => return err.to_compile_error(),
        };
        let field_type = match java_field_type_tokens(&field.ty) {
            Ok(field_type) => field_type,
            Err(err) => return err.to_compile_error(),
        };
        let rust_type = match java_field_rust_type(&field.ty) {
            Ok(rust_type) => rust_type,
            Err(err) => return err.to_compile_error(),
        };

        output.extend(quote! {
            impl ::android::desc::JavaFieldDescriptor for #field_name {
                type ClassDescriptor = #class_ident;
                type RustType = #rust_type;
                const SIGNATURE: &'static str = #signature;
                const FIELD_TYPE: ::android::desc::JavaFieldType = #field_type;
            }
        });
    }

    for method in methods {
        let method_name = method.name.clone();
        if emitted_methods.insert(method_name.to_string()) {
            output.extend(quote! {
                #[allow(non_camel_case_types)]
                pub struct #method_name;
            });
        }

        let arg_tuple = match java_args_tuple(&method.args) {
            Ok(args) => args,
            Err(err) => return err.to_compile_error(),
        };

        let mut arg_signatures = String::new();
        for arg in &method.args {
            match java_type_signature(arg, false) {
                Ok(sig) => arg_signatures.push_str(&sig),
                Err(err) => return err.to_compile_error(),
            }
        }

        let return_signature = match java_type_signature(&method.return_ty, true) {
            Ok(sig) => sig,
            Err(err) => return err.to_compile_error(),
        };
        let return_type = match java_return_type_tokens(&method.return_ty) {
            Ok(return_type) => return_type,
            Err(err) => return err.to_compile_error(),
        };

        let signature = format!("({arg_signatures}){return_signature}");

        output.extend(quote! {
            impl ::android::JavaMethodDescriptor<#arg_tuple> for #method_name {
                type ClassDescriptor = #class_ident;
                const SIGNATURE: &'static str = #signature;
                const RETURN_TYPE: ::android::desc::JavaReturnType = #return_type;
            }
        });
    }

    output
}

fn java_type_signature(ty: &JavaType, allow_void: bool) -> syn::Result<String> {
    match ty {
        JavaType::Void if allow_void => Ok("V".to_string()),
        JavaType::Void => Err(syn::Error::new(
            Span::call_site(),
            "`void` is only allowed as a return type",
        )),
        JavaType::PrimitiveArray(ident) => Ok(format!("[{}", java_primitive_signature(ident)?)),
        JavaType::Primitive(ident) => match ident.to_string().as_str() {
            _ => Ok(java_primitive_signature(ident)?.to_string()),
        },
        JavaType::String => Ok("Ljava/lang/String;".to_string()),
        JavaType::Object(path) => Ok(format!("L{};", path.jni_name())),
    }
}

fn java_arg_rust_type(ty: &JavaType) -> syn::Result<TokenStream> {
    match ty {
        JavaType::Void => Err(syn::Error::new(
            Span::call_site(),
            "`void` is only allowed as a return type",
        )),
        JavaType::PrimitiveArray(ident) => Ok(java_primitive_array_rust_type(ident)?),
        JavaType::Primitive(ident) => Ok(match ident.to_string().as_str() {
            "boolean" => quote! { ::jni::sys::jboolean },
            "byte" => quote! { ::jni::sys::jbyte },
            "char" => quote! { ::jni::sys::jchar },
            "short" => quote! { ::jni::sys::jshort },
            "int" => quote! { ::jni::sys::jint },
            "long" => quote! { ::jni::sys::jlong },
            "float" => quote! { ::jni::sys::jfloat },
            "double" => quote! { ::jni::sys::jdouble },
            _ => {
                return Err(syn::Error::new(
                    ident.span(),
                    format!("unsupported Java primitive type `{ident}`"),
                ))
            }
        }),
        JavaType::String => Ok(quote! { ::jni::sys::jstring }),
        JavaType::Object(_) => Ok(quote! { ::jni::sys::jobject }),
    }
}

fn java_field_rust_type(ty: &JavaType) -> syn::Result<TokenStream> {
    match ty {
        JavaType::Void => Err(syn::Error::new(
            Span::call_site(),
            "`void` is not a valid field type",
        )),
        JavaType::PrimitiveArray(ident) => java_primitive_array_rust_type(ident),
        JavaType::Primitive(ident) => java_arg_rust_type(&JavaType::Primitive(ident.clone())),
        JavaType::String => Ok(quote! { ::jni::sys::jstring }),
        JavaType::Object(_) => Ok(quote! { ::jni::sys::jobject }),
    }
}

fn java_args_tuple(args: &[JavaType]) -> syn::Result<TokenStream> {
    let rust_types: Vec<_> = args
        .iter()
        .map(java_arg_rust_type)
        .collect::<syn::Result<_>>()?;

    Ok(match rust_types.as_slice() {
        [] => quote! { () },
        [only] => quote! { (#only,) },
        _ => quote! { (#(#rust_types),*) },
    })
}

fn java_return_type_tokens(ty: &JavaType) -> syn::Result<TokenStream> {
    match ty {
        JavaType::Void => Ok(quote! {
            ::android::desc::JavaReturnType::Primitive(::android::desc::JavaPrimitiveType::Void)
        }),
        JavaType::Primitive(ident) => match ident.to_string().as_str() {
            "byte" => Ok(quote! {
                ::android::desc::JavaReturnType::Primitive(::android::desc::JavaPrimitiveType::Byte)
            }),
            "char" => Ok(quote! {
                ::android::desc::JavaReturnType::Primitive(::android::desc::JavaPrimitiveType::Char)
            }),
            "int" => Ok(quote! {
                ::android::desc::JavaReturnType::Primitive(::android::desc::JavaPrimitiveType::Int)
            }),
            "float" => Ok(quote! {
                ::android::desc::JavaReturnType::Primitive(::android::desc::JavaPrimitiveType::Float)
            }),
            "boolean" => Ok(quote! {
                ::android::desc::JavaReturnType::Primitive(::android::desc::JavaPrimitiveType::Boolean)
            }),
            "double" => Ok(quote! {
                ::android::desc::JavaReturnType::Primitive(::android::desc::JavaPrimitiveType::Double)
            }),
            _ => Err(syn::Error::new(
                ident.span(),
                format!(
                    "unsupported Java return type `{ident}`; supported return types are void, byte, char, int, float, boolean, double, String, or object types"
                ),
            )),
        },
        JavaType::PrimitiveArray(_) => Err(syn::Error::new(
            Span::call_site(),
            "primitive arrays are not supported as Java method return types",
        )),
        JavaType::String => Ok(quote! { ::android::desc::JavaReturnType::String }),
        JavaType::Object(path) => {
            let class_name = path.jni_name();
            Ok(quote! {
                ::android::desc::JavaReturnType::Object {
                    class_name: #class_name,
                }
            })
        }
    }
}

fn java_field_type_tokens(ty: &JavaType) -> syn::Result<TokenStream> {
    match ty {
        JavaType::Void => Err(syn::Error::new(
            Span::call_site(),
            "`void` is not a valid field type",
        )),
        JavaType::Primitive(ident) => {
            let primitive = java_primitive_type_tokens(ident)?;
            Ok(quote! { ::android::desc::JavaFieldType::Primitive(#primitive) })
        }
        JavaType::PrimitiveArray(ident) => {
            let primitive = java_primitive_type_tokens(ident)?;
            Ok(quote! { ::android::desc::JavaFieldType::PrimitiveArray(#primitive) })
        }
        JavaType::String => Ok(quote! { ::android::desc::JavaFieldType::String }),
        JavaType::Object(path) => {
            let class_name = path.jni_name();
            Ok(quote! {
                ::android::desc::JavaFieldType::Object {
                    class_name: #class_name,
                }
            })
        }
    }
}

fn java_primitive_signature(ident: &syn::Ident) -> syn::Result<&'static str> {
    match ident.to_string().as_str() {
        "boolean" => Ok("Z"),
        "byte" => Ok("B"),
        "char" => Ok("C"),
        "short" => Ok("S"),
        "int" => Ok("I"),
        "long" => Ok("J"),
        "float" => Ok("F"),
        "double" => Ok("D"),
        _ => Err(syn::Error::new(
            ident.span(),
            format!("unsupported Java primitive type `{ident}`"),
        )),
    }
}

fn java_primitive_type_tokens(ident: &syn::Ident) -> syn::Result<TokenStream> {
    match ident.to_string().as_str() {
        "void" => Ok(quote! { ::android::desc::JavaPrimitiveType::Void }),
        "int" => Ok(quote! { ::android::desc::JavaPrimitiveType::Int }),
        "float" => Ok(quote! { ::android::desc::JavaPrimitiveType::Float }),
        "boolean" => Ok(quote! { ::android::desc::JavaPrimitiveType::Boolean }),
        "double" => Ok(quote! { ::android::desc::JavaPrimitiveType::Double }),
        "char" => Ok(quote! { ::android::desc::JavaPrimitiveType::Char }),
        "byte" => Ok(quote! { ::android::desc::JavaPrimitiveType::Byte }),
        _ => Err(syn::Error::new(
            ident.span(),
            format!("unsupported Java primitive type `{ident}`"),
        )),
    }
}

fn java_primitive_array_rust_type(ident: &syn::Ident) -> syn::Result<TokenStream> {
    match ident.to_string().as_str() {
        "int" => Ok(quote! { ::jni::sys::jintArray }),
        "float" => Ok(quote! { ::jni::sys::jfloatArray }),
        "boolean" => Ok(quote! { ::jni::sys::jbooleanArray }),
        "double" => Ok(quote! { ::jni::sys::jdoubleArray }),
        "char" => Ok(quote! { ::jni::sys::jcharArray }),
        "byte" => Ok(quote! { ::jni::sys::jbyteArray }),
        _ => Err(syn::Error::new(
            ident.span(),
            format!(
                "unsupported Java primitive array type `{ident}`; supported array field types are boolean[], byte[], char[], int[], float[], and double[]"
            ),
        )),
    }
}
