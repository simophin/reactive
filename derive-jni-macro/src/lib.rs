mod bridge;

use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;

#[proc_macro_attribute]
#[proc_macro_error]
pub fn java_class(attr: TokenStream, item: TokenStream) -> TokenStream {
    bridge::make_jni_bridge(attr.into(), item.into()).into()
}
