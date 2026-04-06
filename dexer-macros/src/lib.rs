mod codegen;
mod parse;
mod validate;

use proc_macro::TokenStream;

#[proc_macro]
pub fn dex_class(input: TokenStream) -> TokenStream {
    let input2 = proc_macro2::TokenStream::from(input);
    match parse::parse(input2) {
        Err(e) => e.to_compile_error().into(),
        Ok(ast) => match validate::validate(&ast) {
            Err(e) => e.to_compile_error().into(),
            Ok(()) => codegen::generate(ast).into(),
        },
    }
}
