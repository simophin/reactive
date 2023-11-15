use convert_case::{Case, Casing};
use proc_macro2::{Literal, TokenStream};
use proc_macro_error::abort;
use quote::{format_ident, quote};
use syn::{parse2, parse_quote, FnArg, ItemTrait, ReturnType, TraitItem, Type};

use crate::{
    call::build_jni_call_list,
    sig::{build_jni_method_signature, SignatureOutput},
};

pub fn make_jni_bridge(attr: TokenStream, item: TokenStream) -> TokenStream {
    let class_name: Option<Literal> = match parse2(attr) {
        Ok(v) => v,
        Err(v) => panic!("Failed to parse attribute"),
    };

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
                if f.default.is_none() && f.sig.ident.to_string().starts_with("new") =>
            {
                let ReturnType::Default = f.sig.output else {
                    abort!(f, "\"new\" method must not have a return type");
                };

                let Some(class_name) = class_name.as_ref() else {
                    abort!(f, "Missing class name on the 'java_class' attribute");
                };

                match f.sig.inputs.first() {
                    Some(FnArg::Receiver(_)) => {
                        abort!(f, "\"new\" method must not have a Self parameter");
                    }
                    _ => {}
                }

                let return_type: Type = parse_quote! {
                    ::derive_jni::InvocationResult<::jni::objects::AutoLocal<'local, ::jni::objects::JObject<'local>>>
                };

                let builder =
                    build_jni_method_signature(f.sig.inputs.iter(), SignatureOutput::Literal("V"));

                let call_list = build_jni_call_list(f.sig.inputs.iter());

                f.sig.output = ReturnType::Type(Default::default(), Box::new(return_type));
                f.sig
                    .inputs
                    .insert(0, parse_quote! { env: &mut ::jni::JNIEnv<'local> });
                f.sig.generics.params.insert(0, parse_quote! { 'local });

                f.default = Some(parse_quote!( {
                     {
                        let sig = #builder;
                        let args = [ #call_list ];

                        let ret = env.new_object(
                            #class_name,
                            sig.as_str(),
                            &args
                        )?;

                        Ok(::jni::objects::AutoLocal::new(ret, env))
                    }
                }));
            }

            TraitItem::Fn(f)
                if f.default.is_none()
                    && matches!(f.sig.inputs.first(), Some(FnArg::Receiver(_))) =>
            {
                f.sig
                    .inputs
                    .insert(1, parse_quote! { env: &mut ::jni::JNIEnv<'_> });

                let java_method_name = f.sig.ident.to_string().to_case(Case::Camel);
                let orig_output: Type;

                match &mut f.sig.output {
                    ReturnType::Default => {
                        orig_output = parse_quote! { () };
                        f.sig.output = parse_quote! { -> ::derive_jni::InvocationResult<()> };
                    }

                    ReturnType::Type(_, ty) => {
                        orig_output = ty.as_ref().clone();
                        *ty = parse_quote! { ::derive_jni::InvocationResult<#ty> };
                    }
                }

                let builder = build_jni_method_signature(
                    f.sig.inputs.iter().skip(2),
                    SignatureOutput::Type(&orig_output),
                );
                let call_list = build_jni_call_list(f.sig.inputs.iter().skip(2));

                f.default = Some(parse_quote! {
                    {
                        let sig = #builder;
                        let obj = self.get_java_object(env)?;
                        let args = [ #call_list ];

                        let ret = env.call_method(
                            obj,
                            #java_method_name,
                            sig.as_str(),
                            &args
                        )?;

                        use ::derive_jni::ToRustType;

                        match ret.to_rust_type(env) {
                            Ok(v) => Ok(v),
                            Err(e) => Err(::derive_jni::InvocationError::ReturnConvertError(Box::new(e))),
                        }
                    }
                });
            }

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
                fn new();
                fn set_text(&self, text: String);
                fn text(&self) -> String;
                fn set_text_size(&self, size: Option<f32>);
            }
        };

        // println!("{:?}", t);

        let output = make_jni_bridge(parse_quote! { "android/view/View" }, input);

        let output = prettyplease::unparse(&parse_file(&output.to_string()).unwrap());
        println!("{output}");
    }
}
