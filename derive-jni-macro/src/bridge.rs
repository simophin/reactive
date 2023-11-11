use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::ItemTrait;

pub fn make_jni_bridge(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let ItemTrait {
        attrs,
        vis,
        unsafety,
        auto_token,
        ident,
        generics,
        supertraits,
        items,
        ..
    } = syn::parse2(item).unwrap();

    let ident = format_ident!("{}JavaObject", ident);

    quote! {
        #vis trait #ident
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
                fn set_text(&self, text: String);
                fn text(&self) -> String;
            }
        };

        let output = make_jni_bridge(Default::default(), input);

        let output = prettyplease::unparse(&parse_file(&output.to_string()).unwrap());
        println!("{output}");
    }
}
