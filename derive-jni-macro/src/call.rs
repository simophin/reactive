use proc_macro_error::abort;
use quote::quote;
use syn::{parse_quote, punctuated::Punctuated, Expr, PatType, Token, Type};

pub fn build_jni_call_list<'a>(
    args: impl Iterator<Item = &'a PatType> + 'a,
) -> Punctuated<Expr, Token![,]> {
    let mut call_list: Punctuated<_, _> = Default::default();

    for input in args {
        let (is_ref, ty) = match &*input.ty {
            Type::Path(path) => (false, &path.path),
            Type::Reference(reference) => match &*reference.elem {
                Type::Path(path) => (true, &path.path),
                _ => abort!(input, "Expected a type"),
            },
            _ => abort!(input, "Expected a type"),
        };

        let ident = match &*input.pat {
            syn::Pat::Ident(ident) => &ident.ident,
            _ => abort!(input, "Expected a name"),
        };

        let ident = if is_ref {
            quote! { #ident }
        } else {
            quote! { &#ident }
        };

        call_list.push(parse_quote! {
            <#ty as ::derive_jni::ToJavaValue>::into_java_value(#ident, env)
                .map_err(|e| ::derive_jni::InvocationError::ParameterConvertError {
                    name: stringify!(#ident),
                    err: Box::new(e),
                })?
                .into()
        });
    }

    call_list
}
