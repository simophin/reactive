use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use proc_macro_error::abort;
use quote::{format_ident, quote};
use syn::{parse_quote, punctuated::Punctuated, Expr, FnArg, ItemTrait, Token, TraitItem, Type};

pub fn make_jni_bridge(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let ItemTrait {
        attrs,
        vis,
        unsafety,
        auto_token,
        ident,
        generics,
        mut supertraits,
        mut items,
        ..
    } = syn::parse2(item).unwrap();

    let ident = format_ident!("{}JavaObject", ident);
    let where_clause = &generics.where_clause;
    for item in &mut items {
        match item {
            TraitItem::Fn(f)
                if f.default.is_none()
                    && matches!(f.sig.inputs.first(), Some(FnArg::Receiver(_))) =>
            {
                f.sig
                    .inputs
                    .insert(1, parse_quote! { env: &mut ::jni::JNIEnv<'_> });

                let mut builder_code: Vec<TokenStream> = vec![];
                let mut call_list: Punctuated<Expr, Token![,]> = Default::default();

                for input in f.sig.inputs.iter().skip(2) {
                    let FnArg::Typed(input) = input else {
                        abort!(input, "Expected typed argument");
                    };

                    let (is_ref, ty) = match &*input.ty {
                        Type::Path(path) => (false, &path.path),
                        Type::Reference(reference) => match &*reference.elem {
                            Type::Path(path) => (true, &path.path),
                            _ => abort!(input, "Expected a type"),
                        },
                        _ => abort!(input, "Expected a type"),
                    };

                    builder_code.push(quote! {
                         let builder = builder.add_argument::<#ty>();
                    });

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
                        <#ty as ::derive_jni::ToJavaValue>::into_java_value(#ident, env)?
                    });
                }

                let java_method_name = f.sig.ident.to_string().to_case(Case::Camel);

                let orig_output: Type;

                match &mut f.sig.output {
                    syn::ReturnType::Default => {
                        orig_output = parse_quote! { () };
                        f.sig.output = parse_quote! { -> ::derive_jni::InvocationResult<()> };
                    }

                    syn::ReturnType::Type(_, ty) => {
                        orig_output = ty.as_ref().clone();
                        *ty = parse_quote! { ::derive_jni::InvocationResult<#ty> };
                    }
                }

                f.default = Some(parse_quote! {
                    {
                        let sig = {
                            let builder = ::derive_jni::MethodSignatureBuilder::new();
                            #(#builder_code;)*
                            builder.build::<#orig_output>()
                        };

                        let ret = env.call_method(
                            self.get_java_object()?,
                            #java_method_name,
                            sig.as_str(),
                            &[ #call_list ]
                        )?;

                        use ::derive_jni::ToRustType;

                        match ret.to_rust_type(env) {
                            Ok(v) => Ok(v),
                            Err(e) => Err(::derive_jni::InvocationError::ReturnConvertError(Box::new(e))),
                        }
                    }
                });
            }

            TraitItem::Fn(f) if f.default.is_none() && f.sig.ident.to_string() == "new" => {}

            _ => {}
        }
    }

    supertraits.insert(0, parse_quote! { ::derive_jni::WithJavaObject });

    quote! {
        #vis trait #ident #generics : #supertraits #where_clause {
            #(#items)*
        }
    }
}

#[cfg(test)]
mod tests {

    use syn::{parse2, parse_file, parse_quote};

    use super::*;

    #[test]
    fn parsing_works() {
        let input = quote! {
            trait View: Clone + 'static {
                fn set_text(&self, text: String);
                fn text(&self) -> String;
                fn set_text_size(&self, size: Option<f32>);
            }
        };

        // println!("{:?}", t);

        let output = make_jni_bridge(Default::default(), input);

        let output = prettyplease::unparse(&parse_file(&output.to_string()).unwrap());
        println!("{output}");
    }
}
