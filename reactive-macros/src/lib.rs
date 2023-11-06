use proc_macro2::TokenStream;
use proc_macro_error::{abort, proc_macro_error};
use quote::{format_ident, quote, ToTokens};
use rstml::{
    node::{Node, NodeAttribute, NodeBlock, NodeElement, NodeFragment},
    parse2,
};

#[proc_macro]
#[proc_macro_error]
pub fn jsx(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    jsx2(input.into()).into()
}

fn jsx2(input: TokenStream) -> TokenStream {
    let nodes = match parse2(input) {
        Ok(nodes) => nodes,
        Err(e) => return e.to_compile_error(),
    };

    let component_nodes: Vec<_> = nodes.iter().map_while(ComponentNode::new).collect();

    match component_nodes.len() {
        0 => "()".parse().unwrap(),
        1 => component_nodes[0].to_token_stream(),
        _ => abort!(
            component_nodes
                .into_iter()
                .fold(TokenStream::new(), |mut acc, n| {
                    n.raw_token(&mut acc);
                    acc
                }),
            "Only one root node is allowed. To use multiple nodes, wrap them in a fragment."
        ),
    }
}

enum ComponentNode<'a> {
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

struct ComponentAttribute<'a>(&'a NodeAttribute);

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

struct ComponentChildren<'a>(&'a Vec<Node>);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsing_works() {
        let output = jsx2(quote! {
            <MyView attr1 attr2="hello">
                <Child1 />
                <Child2 attr3="world" />
            </MyView>
        });

        assert_eq!(
            output.to_string(),
            quote! {
                {
                    let mut builder = MyViewBuilder::default();
                    let mut builder = builder.attr1(true);
                    let mut builder = builder.attr2("hello");
                    let mut builder = builder.children(vec![
                        {
                            let mut builder = Child1Builder::default();
                            builder.build().unwrap()
                        },
                        {
                            let mut builder = Child2Builder::default();
                            let mut builder = builder.attr3("world");
                            builder.build().unwrap()
                        },
                    ]);
                    builder.build().unwrap()
                }
            }
            .to_string()
        );
    }
}
