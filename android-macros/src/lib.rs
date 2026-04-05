mod declare_jni_binding;

use proc_macro::TokenStream;
use syn::parse_macro_input;

#[proc_macro]
pub fn declare_jni_binding(input: TokenStream) -> TokenStream {
    let binding = parse_macro_input!(input as declare_jni_binding::ast::JavaBinding);
    declare_jni_binding::expand::expand(binding).into()
}
