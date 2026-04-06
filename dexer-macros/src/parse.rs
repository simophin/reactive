use proc_macro2::{Span, TokenStream};
use syn::{
    braced, parenthesized,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token, Attribute, Error, FnArg, Ident, LitStr, PatType, Result, ReturnType, Token, Type,
    Visibility,
};

// ─────────────────────────── public AST ──────────────────────────────────

pub struct DexClass {
    pub java_class: LitStr,
    pub vis: Visibility,
    pub name: Ident,
    pub fields: Vec<RustField>,
    pub extends: LitStr,
    pub implements: Vec<LitStr>,
    pub methods: Vec<DexMethod>,
}

pub struct RustField {
    pub name: Ident,
    pub ty: Type,
}

pub struct DexMethod {
    pub kind: MethodKind,
    pub rust_name: Ident,
    /// Parameters after stripping `&mut self`, `env`, and `super_`.
    /// These map 1-to-1 with JNI parameters.
    pub jni_params: Vec<JniParam>,
    pub return_ty: ReturnType,
    pub body: syn::Block,
}

pub enum MethodKind {
    Constructor,
    Override { java_name: LitStr },
    Method { java_name: LitStr },
}

pub struct JniParam {
    pub name: Ident,
    pub ty: Type,
    /// Value of `#[class = "..."]` if present.
    pub class_attr: Option<LitStr>,
}

// ─────────────────────────── entry point ─────────────────────────────────

pub fn parse(input: TokenStream) -> Result<DexClass> {
    syn::parse2(input)
}

// ─────────────────────────── impl Parse ──────────────────────────────────

impl Parse for DexClass {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        // ── #[java_class = "..."] ─────────────────────────────────────────
        let attrs = Attribute::parse_outer(input)?;
        let java_class = extract_java_class(&attrs)?;

        // ── pub struct Name { fields } ────────────────────────────────────
        let vis: Visibility = input.parse()?;
        let _struct_kw: Token![struct] = input.parse()?;
        let name: Ident = input.parse()?;

        let fields = if input.peek(token::Brace) {
            let content;
            braced!(content in input);
            parse_rust_fields(&content)?
        } else {
            let _semi: Token![;] = input.parse()?;
            Vec::new()
        };

        // ── extends "..." ; ───────────────────────────────────────────────
        let extends_kw: Ident = input.parse()?;
        if extends_kw != "extends" {
            return Err(Error::new(extends_kw.span(), "expected `extends`"));
        }
        let extends: LitStr = input.parse()?;
        let _: Token![;] = input.parse()?;

        // ── implements "..." ; (zero or more) ─────────────────────────────
        let mut implements = Vec::new();
        while input.peek(Ident) && input.fork().parse::<Ident>().map(|i| i == "implements").unwrap_or(false) {
            let _: Ident = input.parse()?; // "implements"
            let iface: LitStr = input.parse()?;
            let _: Token![;] = input.parse()?;
            implements.push(iface);
        }

        // ── method defs ───────────────────────────────────────────────────
        let mut methods = Vec::new();
        while !input.is_empty() {
            methods.push(input.parse()?);
        }

        Ok(DexClass { java_class, vis, name, fields, extends, implements, methods })
    }
}

fn extract_java_class(attrs: &[Attribute]) -> Result<LitStr> {
    for attr in attrs {
        if attr.path().is_ident("java_class") {
            let lit: LitStr = attr.parse_args()?;
            return Ok(lit);
        }
    }
    Err(Error::new(Span::call_site(), "missing #[java_class = \"...\"] attribute"))
}

fn parse_rust_fields(input: ParseStream<'_>) -> Result<Vec<RustField>> {
    let mut fields = Vec::new();
    while !input.is_empty() {
        // Strip field-level attributes
        let _attrs = Attribute::parse_outer(input)?;
        let name: Ident = input.parse()?;
        let _: Token![:] = input.parse()?;
        let ty: Type = input.parse()?;
        if input.peek(Token![,]) {
            let _: Token![,] = input.parse()?;
        }
        fields.push(RustField { name, ty });
    }
    Ok(fields)
}

// ─────────────────────────── method parsing ──────────────────────────────

impl Parse for DexMethod {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let outer_attrs = Attribute::parse_outer(input)?;
        let kind = parse_method_kind(&outer_attrs)?;

        // fn name( ... ) -> ReturnType { body }
        let _fn_kw: Token![fn] = input.parse()?;
        let rust_name: Ident = input.parse()?;

        let params_content;
        parenthesized!(params_content in input);
        let raw_params: Punctuated<FnArg, Token![,]> =
            Punctuated::parse_terminated(&params_content)?;

        let return_ty: ReturnType = input.parse()?;
        let body: syn::Block = input.parse()?;

        let jni_params = extract_jni_params(raw_params, &kind)?;

        Ok(DexMethod { kind, rust_name, jni_params, return_ty, body })
    }
}

fn parse_method_kind(attrs: &[Attribute]) -> Result<MethodKind> {
    for attr in attrs {
        let path = attr.path();
        if path.is_ident("constructor") {
            return Ok(MethodKind::Constructor);
        }
        if path.is_ident("override") {
            let java_name = attr.parse_args::<NameArg>()?.0;
            return Ok(MethodKind::Override { java_name });
        }
        if path.is_ident("method") {
            let java_name = attr.parse_args::<NameArg>()?.0;
            return Ok(MethodKind::Method { java_name });
        }
    }
    Err(Error::new(
        Span::call_site(),
        "method must be annotated with #[constructor], #[override(name = \"...\")], or #[method(name = \"...\")]",
    ))
}

/// Parses `name = "some.string"` inside an attribute.
struct NameArg(LitStr);
impl Parse for NameArg {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let kw: Ident = input.parse()?;
        if kw != "name" {
            return Err(Error::new(kw.span(), "expected `name = \"...\"`"));
        }
        let _: Token![=] = input.parse()?;
        Ok(NameArg(input.parse()?))
    }
}

/// Extract JNI parameters by skipping `&mut self`, the `env` param, and `super_`.
fn extract_jni_params(
    raw: Punctuated<FnArg, Token![,]>,
    kind: &MethodKind,
) -> Result<Vec<JniParam>> {
    let mut params = Vec::new();
    let mut iter = raw.into_iter();

    // 1. Skip `&mut self` / `&self`
    match iter.next() {
        Some(FnArg::Receiver(_)) => {}
        Some(other) => {
            // constructor has no self
            if !matches!(kind, MethodKind::Constructor) {
                return Err(Error::new_spanned(other, "expected `&mut self` as first parameter"));
            }
            // For constructor, the first param IS a jni param — re-process it
            if let FnArg::Typed(pt) = other {
                // But first check if it's the env param
                if !is_env_param(&pt) {
                    params.push(make_jni_param(pt)?);
                    // skip env next
                    if let Some(FnArg::Typed(env_pt)) = iter.next() {
                        if !is_env_param(&env_pt) {
                            params.push(make_jni_param(env_pt)?);
                        }
                    }
                    // rest are JNI params
                    for arg in iter {
                        if let FnArg::Typed(pt) = arg {
                            params.push(make_jni_param(pt)?);
                        }
                    }
                    return Ok(params);
                }
            }
        }
        None => {}
    }

    // 2. Skip `env: &mut jni::JNIEnv` / `env: &jni::JNIEnv`
    if let Some(arg) = iter.next() {
        if let FnArg::Typed(pt) = arg {
            if !is_env_param(&pt) {
                return Err(Error::new_spanned(pt.ty, "second parameter must be `env: &mut jni::JNIEnv`"));
            }
        }
    }

    // 3. For overrides, skip `super_: dexer::SuperCaller`
    if matches!(kind, MethodKind::Override { .. }) {
        if let Some(arg) = iter.next() {
            if let FnArg::Typed(pt) = arg {
                if !is_super_caller_param(&pt) {
                    return Err(Error::new_spanned(
                        pt.ty,
                        "third parameter of an #[override] method must be `super_: dexer::SuperCaller`",
                    ));
                }
            }
        }
    }

    // 4. Remaining params are JNI params
    for arg in iter {
        match arg {
            FnArg::Typed(pt) => params.push(make_jni_param(pt)?),
            FnArg::Receiver(r) => {
                return Err(Error::new_spanned(r, "unexpected receiver in JNI param list"))
            }
        }
    }

    Ok(params)
}

fn is_env_param(pt: &PatType) -> bool {
    // Matches `env: &mut jni::JNIEnv` or `env: &jni::JNIEnv`
    if let syn::Pat::Ident(pi) = pt.pat.as_ref() {
        if pi.ident != "env" { return false; }
    }
    // Accept any reference type as the env
    matches!(pt.ty.as_ref(), Type::Reference(_))
}

fn is_super_caller_param(pt: &PatType) -> bool {
    if let syn::Pat::Ident(pi) = pt.pat.as_ref() {
        pi.ident == "super_"
    } else {
        false
    }
}

fn make_jni_param(pt: PatType) -> Result<JniParam> {
    let name = match pt.pat.as_ref() {
        syn::Pat::Ident(pi) => pi.ident.clone(),
        other => return Err(Error::new_spanned(other, "JNI parameter must be a simple identifier")),
    };

    // Extract `#[class = "..."]` from param attrs
    let class_attr = extract_class_attr(&pt.attrs)?;

    Ok(JniParam { name, ty: *pt.ty, class_attr })
}

fn extract_class_attr(attrs: &[Attribute]) -> Result<Option<LitStr>> {
    for attr in attrs {
        if attr.path().is_ident("class") {
            let lit: LitStr = attr.parse_args()?;
            return Ok(Some(lit));
        }
    }
    Ok(None)
}
