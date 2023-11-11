use proc_macro2::TokenStream;
use proc_macro_error::abort;
use quote::{quote, ToTokens};
use rstml::node::NodeAttribute;

pub struct ComponentAttribute<'a>(pub &'a NodeAttribute);

impl<'a> ToTokens for ComponentAttribute<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let NodeAttribute::Attribute(attr) = self.0 else {
            abort!(self.0, "Only static attribute is supported");
        };

        let key = &attr.key;
        let value = attr
            .value()
            .map(|s| s.to_token_stream())
            .unwrap_or_else(|| quote! { true });

        // let (key, value) = self.0;
        tokens.extend(quote! {
            let mut builder = builder.#key(#value);
        });
    }
}
