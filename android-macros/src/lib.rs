use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    braced,
    parse::{Parse, ParseStream},
    parse_macro_input, Ident, LitStr, Token,
};

struct ViewProps {
    class: LitStr,
    props: Vec<Prop>,
}

struct Prop {
    name: Ident,
    ty: Ident,
}

impl Parse for ViewProps {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let kw: Ident = input.parse()?;
        if kw != "class" {
            return Err(syn::Error::new(kw.span(), "expected `class`"));
        }
        let class: LitStr = input.parse()?;
        let content;
        braced!(content in input);
        let mut props = Vec::new();
        while !content.is_empty() {
            let name: Ident = content.parse()?;
            content.parse::<Token![:]>()?;
            let ty: Ident = content.parse()?;
            let _ = content.parse::<Token![,]>();
            props.push(Prop { name, ty });
        }
        Ok(ViewProps { class, props })
    }
}

/// Maps a JNI type name to its descriptor character(s) within a method signature.
fn jni_param_sig(ty: &str) -> Option<&'static str> {
    match ty {
        "jboolean" => Some("Z"),
        "jbyte" => Some("B"),
        "jchar" => Some("C"),
        "jshort" => Some("S"),
        "jint" => Some("I"),
        "jlong" => Some("J"),
        "jfloat" => Some("F"),
        "jdouble" => Some("D"),
        "jstring" => Some("Ljava/lang/String;"),
        "jobject" => Some("Ljava/lang/Object;"),
        _ => None,
    }
}

fn jni_rust_type(ty: &str) -> proc_macro2::TokenStream {
    match ty {
        "jboolean" => quote! { ::jni::sys::jboolean },
        "jbyte" => quote! { ::jni::sys::jbyte },
        "jchar" => quote! { ::jni::sys::jchar },
        "jshort" => quote! { ::jni::sys::jshort },
        "jint" => quote! { ::jni::sys::jint },
        "jlong" => quote! { ::jni::sys::jlong },
        "jfloat" => quote! { ::jni::sys::jfloat },
        "jdouble" => quote! { ::jni::sys::jdouble },
        "jstring" => quote! { ::jni::sys::jstring },
        "jobject" => quote! { ::jni::sys::jobject },
        _ => unreachable!(),
    }
}

/// `textColor` → `"setTextColor"`
fn to_setter(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        None => "set".to_string(),
        Some(first) => format!("set{}{}", first.to_uppercase(), chars.as_str()),
    }
}

/// `textColor` → `TEXT_COLOR`
fn to_screaming_snake(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.extend(c.to_uppercase());
    }
    out
}

/// Generates a `PropDescriptor` constant for each property in a class.
///
/// ```ignore
/// view_props! {
///     class "android/widget/TextView" {
///         text: jstring,
///         textColor: jint,
///         myObj: jobject,
///     }
/// }
/// ```
///
/// Expands to one `pub const` per property, named in SCREAMING_SNAKE_CASE,
/// with the setter method name and JNI signature derived from the field name
/// and JNI type respectively.
#[proc_macro]
pub fn view_props(input: TokenStream) -> TokenStream {
    let ViewProps { class, props } = parse_macro_input!(input as ViewProps);
    let class_str = class.value();

    let mut output = proc_macro2::TokenStream::new();

    for prop in props {
        let ty_str = prop.ty.to_string();

        let param_sig = match jni_param_sig(&ty_str) {
            Some(s) => s,
            None => {
                return syn::Error::new(
                    prop.ty.span(),
                    format!("unknown JNI type `{ty_str}`; use jboolean, jbyte, jchar, jshort, jint, jlong, jfloat, jdouble, jstring, or jobject"),
                )
                .to_compile_error()
                .into();
            }
        };

        let full_sig = format!("({param_sig})V");
        let method_name = to_setter(&prop.name.to_string());
        let const_name = Ident::new(&to_screaming_snake(&prop.name.to_string()), Span::call_site());
        let rust_type = jni_rust_type(&ty_str);

        output.extend(quote! {
            pub const #const_name: ::android::PropDescriptor<#rust_type> =
                ::android::PropDescriptor::new(#class_str, #method_name, #full_sig);
        });
    }

    output.into()
}
