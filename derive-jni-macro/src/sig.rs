use proc_macro2::TokenStream;
use proc_macro_error::abort;
use quote::quote;
use syn::{ExprBlock, parse_quote, PatType, Type};

pub enum SignatureOutput<'a> {
    Type(&'a Type),
    Literal(&'a str),
}

pub fn build_jni_method_signature<'a>(
    args: impl Iterator<Item = &'a PatType> + 'a,
    output: SignatureOutput<'a>,
) -> ExprBlock {
    let mut tokens = TokenStream::new();

    for input in args {
        let ty = match &*input.ty {
            Type::Path(path) => &path.path,
            Type::Reference(reference) => match &*reference.elem {
                Type::Path(path) => &path.path,
                _ => abort!(input, "Expected a type"),
            },
            _ => abort!(input, "Expected a type"),
        };

        tokens.extend(quote! {
             let builder = builder.add_argument::<#ty>();
        });
    }

    match output {
        SignatureOutput::Type(ty) => {
            parse_quote! {
                {
                    let builder = ::derive_jni::MethodSignatureBuilder::new();
                    #tokens
                    builder.build::<#ty>()
                }
            }
        }

        SignatureOutput::Literal(lit) => {
            parse_quote! {
                {
                    let builder = ::derive_jni::MethodSignatureBuilder::new();
                    #tokens
                    builder.build_with(#lit)
                }
            }
        }
    }
}
