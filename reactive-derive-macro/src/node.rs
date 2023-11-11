use proc_macro2::TokenStream;
use proc_macro_error::abort;
use quote::{format_ident, quote, ToTokens};
use rstml::node::{Node, NodeBlock, NodeElement, NodeFragment};
use syn::spanned::Spanned;

use crate::{attr::ComponentAttribute, children::ComponentChildren};

pub enum ComponentNode<'a> {
    Element(&'a NodeElement),
    Block(&'a NodeBlock),
    Fragment(&'a NodeFragment),
}

impl<'a> ComponentNode<'a> {
    pub fn new(node: &'a Node) -> Option<Self> {
        match node {
            Node::Element(element) => Some(Self::Element(element)),
            Node::Block(block) => Some(Self::Block(block)),
            Node::Fragment(fragment) => Some(Self::Fragment(fragment)),
            Node::Comment(_) => None,
            Node::Doctype(n) => abort!(n, "Doctype node is not supported"),
            Node::Text(n) => abort!(
                n,
                "Freeform text is not supported, you must use a Text component instead"
            ),
            Node::RawText(n) => abort!(
                n,
                "Freeform text is not supported, you must use a Text component instead"
            ),
        }
    }

    pub fn raw_token(&self, out: &mut TokenStream) {
        match self {
            Self::Element(element) => element.to_tokens(out),
            Self::Block(block) => block.to_tokens(out),
            Self::Fragment(fragment) => fragment.to_tokens(out),
        }
    }
}

impl<'a> ToTokens for ComponentNode<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Element(element) => {
                let builder_name = format_ident!("{}Builder", element.open_tag.name.to_string());
                let attributes = element.attributes().iter().map(|s| ComponentAttribute(s));
                let children = ComponentChildren(&element.children);
                tokens.extend(quote! {
                    {
                        let mut builder = #builder_name::default();
                        #(#attributes)*
                        #children

                        builder.build().unwrap()
                    }
                });
            }
            Self::Block(block) => block.to_tokens(tokens),
            Self::Fragment(fragment) => fragment.to_tokens(tokens),
        }
    }
}
