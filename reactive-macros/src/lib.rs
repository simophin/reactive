use proc_macro_error::proc_macro_error;

mod attr;
mod children;
mod jsx;
mod node;

#[proc_macro]
#[proc_macro_error]
pub fn jsx(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    jsx::jsx2(input.into()).into()
}
