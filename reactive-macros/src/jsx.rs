use super::node::ComponentNode;
use proc_macro2::TokenStream;
use proc_macro_error::abort;
use quote::ToTokens;
use rstml::parse2;

pub fn jsx2(input: TokenStream) -> TokenStream {
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

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

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
