use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use rstml::node::Node;

use crate::node::ComponentNode;

pub struct ComponentChildren<'a>(pub &'a Vec<Node>);

impl<'a> ToTokens for ComponentChildren<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let children = self
            .0
            .iter()
            .map_while(ComponentNode::new)
            .collect::<Vec<_>>();

        match children.len() {
            0 => return,
            1 => {
                let child = children.into_iter().next().unwrap();
                tokens.extend(quote! {
                    let mut builder = builder.child(#child);
                });
            }
            _ => {
                tokens.extend(quote! {
                    let mut builder = builder.children(vec![
                        #(#children ,)*
                    ]);
                });
            }
        }
    }
}
