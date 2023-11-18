use std::process::abort;
use convert_case::{Case, Casing};
use proc_macro2::{Ident, Literal, TokenStream};
use proc_macro_error::abort;
use quote::{format_ident, quote};
use syn::{parse2, parse_quote, FnArg, ItemTrait, ReturnType, TraitItem, Type, PatType, Token, TraitItemFn, Signature, TypePath, TypeTuple, Expr, PathSegment};
use syn::punctuated::Punctuated;

use crate::{
    call::build_jni_call_list,
    sig::{build_jni_method_signature, SignatureOutput},
};


struct JavaConstructor {
    struct_ident: Ident,
    ident: Ident,
    class_name: Literal,
    args: Vec<PatType>,
}

impl JavaConstructor {
    fn build_method(&self) -> TokenStream {
        let Self { ident, class_name, args, struct_ident } = self;
        let jni_call_list = build_jni_call_list(args.iter());

        let jni_method_signature = build_jni_method_signature(
            args.iter(),
            SignatureOutput::Literal("V"),
        );

        quote! {
            impl<'a> #struct_ident<'a, jni::objects::AutoLocal<'a, ::jni::objects::JObject<'a>>> {
                pub fn #ident(env: &mut ::jni::JNIEnv<'a>, #(#args,)*)
                    -> ::derive_jni::InvocationResult<Self> {
                    let sig = #jni_method_signature;
                    let call_list = [#jni_call_list];
                    let r = env.new_object(#class_name, sig, &call_list)?;
                    let r = jni::objects::AutoLocal::new(r, env);
                    Ok(Self(r, Default::default()))
                }
            }
        }
    }
}

struct JavaStaticMethod {
    struct_ident: Ident,
    ident: Ident,
    class_name: Literal,
    args: Vec<PatType>,
    return_type: Type,
}

impl JavaStaticMethod {
    fn build_method(&self) -> TokenStream {
        let Self { ident, class_name, args, return_type, struct_ident } = self;
        let jni_call_list = build_jni_call_list(args.iter());

        let jni_method_signature = build_jni_method_signature(
            args.iter(),
            SignatureOutput::Type(return_type),
        );

        let name = ident.to_string().to_case(Case::Camel);

        parse_quote! {
            impl<T> #struct_ident<'_, T> {
                pub fn #ident(env: &mut ::jni::JNIEnv<'a>, #(#args,)*) -> ::derive_jni::InvocationResult<#return_type> {
                    let sig = #jni_method_signature;
                    let call_list = [#jni_call_list];
                    let r = env.call_static_method(#class_name, #name, sig, &call_list)?;

                    use ::derive_jni::ToRustType;
                    r.to_rust_type(env)
                        .map_err(|e| ::derive_jni::InvocationError::ReturnConvertError(Box::new(e)))
                }
            }
        }
    }
}

struct JavaMethod {
    struct_ident: Ident,
    ident: Ident,
    consumes_self: bool,
    args: Vec<PatType>,
    return_type: Type,
}

impl JavaMethod {
    fn build_method(&self) -> TokenStream {
        let Self { ident, consumes_self, args, return_type, struct_ident } = self;
        let jni_call_list = build_jni_call_list(args.iter());

        let jni_method_signature = build_jni_method_signature(
            args.iter(),
            SignatureOutput::Type(return_type),
        );

        let name = ident.to_string().to_case(Case::Camel);
        let self_binding = if *consumes_self {
            quote! { self }
        } else {
            quote! { &'a self }
        };

        parse_quote! {
            impl<'a, T : AsRef<::jni::objects::JObject<'a>>> #struct_ident<'a, T> {
                pub fn #ident(#self_binding, env: &mut ::jni::JNIEnv<'a>, #(#args,)*) -> ::derive_jni::InvocationResult<#return_type> {
                    let sig = #jni_method_signature;
                    let call_list = [#jni_call_list];

                    let r = env.call_method(self.0.as_ref(), #name, sig, &call_list)?;
                    use ::derive_jni::ToRustType;
                    r.to_rust_type(env)
                        .map_err(|e| ::derive_jni::InvocationError::ReturnConvertError(Box::new(e)))
                }
            }
        }
    }
}

enum JavaMethodType {
    Constructor(JavaConstructor),
    StaticMethod(JavaStaticMethod),
    Method(JavaMethod),
}

impl JavaMethodType {
    fn new(item: TraitItemFn, class_name: Option<&Literal>, struct_ident: Ident) -> Result<Self, &'static str> {
        let ident = item.sig.ident.clone();
        let output_is_self: bool;
        let return_or_unit: Type;

        match &item.sig.output {
            ReturnType::Default => {
                output_is_self = false;
                return_or_unit = Type::Tuple(parse_quote! { () });
            }

            ReturnType::Type(_, t) => {
                output_is_self = type_is_self(t.as_ref());
                return_or_unit = t.as_ref().clone();
            }
        };

        match &item.sig.inputs.first() {
            Some(FnArg::Receiver(r)) => {
                Ok(JavaMethodType::Method(JavaMethod {
                    struct_ident,
                    ident,
                    consumes_self: r.reference.is_none(),
                    args: input_as_pat_types(item.sig.inputs.into_iter().skip(1)).collect(),
                    return_type: return_or_unit,
                }))
            }

            _ if ident.to_string().starts_with("new") && output_is_self => {
                let class_name = match class_name {
                    Some(n) => n.clone(),
                    None => abort!(item, "To use constructors, you must specify the class name in the java_class attribute"),
                };

                Ok(JavaMethodType::Constructor(JavaConstructor {
                    struct_ident,
                    ident,
                    args: input_as_pat_types(item.sig.inputs.into_iter()).collect(),
                    class_name,
                }))
            }

            _ => {
                let class_name = match class_name {
                    Some(n) => n.clone(),
                    None => abort!(item, "To use static methods, you must specify the class name in the java_class attribute"),
                };

                Ok(JavaMethodType::StaticMethod(JavaStaticMethod {
                    struct_ident,
                    ident,
                    args: input_as_pat_types(item.sig.inputs.into_iter()).collect(),
                    return_type: return_or_unit,
                    class_name,
                }))
            }
        }
    }

    fn build_rust_method(&self) -> TokenStream {
        match self {
            JavaMethodType::Constructor(c) => c.build_method(),
            JavaMethodType::StaticMethod(m) => m.build_method(),
            JavaMethodType::Method(m) => m.build_method(),
        }
    }
}

fn type_is_self(t: &Type) -> bool {
    matches!(t, Type::Path(TypePath { path, .. }) if path.is_ident("Self"))
}

fn input_as_pat_types(iter: impl Iterator<Item=FnArg>) -> impl Iterator<Item=PatType> {
    iter.map(|arg| match arg {
        FnArg::Typed(t) => t,
        FnArg::Receiver(_) => abort!(arg, "Receiver is not supported at this position"),
    })
}

pub fn make_jni_bridge(attr: TokenStream, item: TokenStream) -> TokenStream {
    let class_name: Option<Literal> = match parse2(attr.clone()) {
        Ok(v) => v,
        Err(_) => panic!("Failed to parse attribute"),
    };

    let ItemTrait {
        vis,
        ident,
        generics,
        supertraits,
        items,
        ..
    } = parse2(item).unwrap();

    if !generics.params.is_empty() {
        abort!(generics.params, "Generic traits are not supported");
    }

    if generics.where_clause.is_some() {
        abort!(generics.where_clause, "Where clauses are not supported");
    }

    if !supertraits.is_empty() {
        abort!(supertraits, "Supertraits are not supported");
    }

    let struct_name = format_ident!("{}JavaObject", ident);

    let methods = items.into_iter().map(|item| {
        match item {
            TraitItem::Fn(f) => {
                match JavaMethodType::new(f.clone(), class_name.as_ref(), struct_name.clone()) {
                    Ok(v) => v.build_rust_method(),
                    Err(e) => abort!(f, format!("Unsupported trait method: {e}")),
                }
            }
            f => abort!(f, "Unsupported trait item"),
        }
    });

    quote! {
        #vis struct #struct_name<'a, T>(pub T, pub std::marker::PhantomData<&'a ()>);

        #(#methods)*
    }
}

#[cfg(test)]
mod tests {
    use syn::{parse2, parse_file, parse_quote};

    use super::*;

    #[test]
    fn parsing_works() {
        let input = quote! {
            trait View {
                fn new_with_mills(mills: i64) -> Self;
                fn new() -> Self;
                fn get_month(&self) -> i32;
                fn hash_code(&self) -> i32;
                fn to_string(&self) -> Option<String>;
            }
        };

        // println!("{:?}", t);

        let output = make_jni_bridge(parse_quote! { "android/view/View" }, input);

        let output = prettyplease::unparse(&parse_file(&output.to_string()).unwrap());
        println!("{output}");
    }
}
