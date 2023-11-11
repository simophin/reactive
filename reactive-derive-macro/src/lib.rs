use proc_macro_error::proc_macro_error;

mod attr;
mod builder;
mod children;
mod jsx;
mod node;

#[proc_macro]
#[proc_macro_error]
pub fn jsx(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    jsx::jsx2(input.into()).into()
}

#[proc_macro_attribute]
#[proc_macro_error]
pub fn component(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    builder::component(attr.into(), item.into()).into()
}
