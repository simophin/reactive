use proc_macro2::{Span, TokenStream};
use syn::{
    braced, parenthesized,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token, Attribute, Error, FnArg, Ident, LitStr, PatType, Result, ReturnType, Token, Type,
    Visibility,
};

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
    pub class_attr: Option<LitStr>,
}

pub fn parse(input: TokenStream) -> Result<DexClass> {
    syn::parse2(input)
}

impl Parse for DexClass {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let attrs = Attribute::parse_outer(input)?;
        let java_class = extract_java_class(&attrs)?;

        let vis: Visibility = input.parse()?;
        let _: Token![struct] = input.parse()?;
        let name: Ident = input.parse()?;

        let fields = if input.peek(token::Brace) {
            let content;
            braced!(content in input);
            parse_rust_fields(&content)?
        } else {
            let _: Token![;] = input.parse()?;
            Vec::new()
        };

        let extends_kw: Ident = input.parse()?;
        if extends_kw != "extends" {
            return Err(Error::new(extends_kw.span(), "expected `extends`"));
        }
        let extends: LitStr = input.parse()?;
        let _: Token![;] = input.parse()?;

        let mut implements = Vec::new();
        while input.peek(Ident)
            && input
                .fork()
                .parse::<Ident>()
                .map(|ident| ident == "implements")
                .unwrap_or(false)
        {
            let _: Ident = input.parse()?;
            let iface: LitStr = input.parse()?;
            let _: Token![;] = input.parse()?;
            implements.push(iface);
        }

        let mut methods = Vec::new();
        while !input.is_empty() {
            methods.push(input.parse()?);
        }

        Ok(DexClass {
            java_class,
            vis,
            name,
            fields,
            extends,
            implements,
            methods,
        })
    }
}

fn extract_java_class(attrs: &[Attribute]) -> Result<LitStr> {
    for attr in attrs {
        if attr.path().is_ident("java_class") {
            if let Ok(lit) = attr.parse_args::<LitStr>() {
                return Ok(lit);
            }
            if let Ok(meta) = attr.meta.require_name_value() {
                if let syn::Expr::Lit(expr_lit) = &meta.value {
                    if let syn::Lit::Str(lit) = &expr_lit.lit {
                        return Ok(lit.clone());
                    }
                }
            }
        }
    }

    Err(Error::new(
        Span::call_site(),
        "missing #[java_class = \"...\"] attribute",
    ))
}

fn parse_rust_fields(input: ParseStream<'_>) -> Result<Vec<RustField>> {
    let mut fields = Vec::new();
    while !input.is_empty() {
        let _attrs = Attribute::parse_outer(input)?;
        if input.peek(Token![pub]) {
            let _: Token![pub] = input.parse()?;
            if input.peek(token::Paren) {
                let content;
                parenthesized!(content in input);
            }
        }
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

impl Parse for DexMethod {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let outer_attrs = Attribute::parse_outer(input)?;
        let kind = parse_method_kind(&outer_attrs)?;

        if input.peek(Token![pub]) {
            let _: Token![pub] = input.parse()?;
            if input.peek(token::Paren) {
                let content;
                parenthesized!(content in input);
            }
        }

        let _: Token![fn] = input.parse()?;
        let rust_name: Ident = input.parse()?;

        let params_content;
        parenthesized!(params_content in input);
        let raw_params: Punctuated<FnArg, Token![,]> =
            Punctuated::parse_terminated(&params_content)?;

        let return_ty: ReturnType = input.parse()?;
        let body: syn::Block = input.parse()?;
        let jni_params = extract_jni_params(raw_params, &kind)?;

        Ok(DexMethod {
            kind,
            rust_name,
            jni_params,
            return_ty,
            body,
        })
    }
}

fn parse_method_kind(attrs: &[Attribute]) -> Result<MethodKind> {
    for attr in attrs {
        let path = attr.path();
        if path.is_ident("constructor") {
            return Ok(MethodKind::Constructor);
        }
        if path.is_ident("override") || path.is_ident("method_override") {
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

fn extract_jni_params(
    raw: Punctuated<FnArg, Token![,]>,
    kind: &MethodKind,
) -> Result<Vec<JniParam>> {
    let mut params = Vec::new();
    let mut args: std::collections::VecDeque<FnArg> = raw.into_iter().collect();

    let mut consumed_env = false;

    match args.pop_front() {
        Some(FnArg::Receiver(_)) => {}
        Some(other) => {
            if !matches!(kind, MethodKind::Constructor) {
                return Err(Error::new_spanned(
                    other,
                    "expected `&mut self` as first parameter",
                ));
            }
            if let FnArg::Typed(pt) = other {
                if is_env_param(&pt) {
                    consumed_env = true;
                } else {
                    params.push(make_jni_param(pt)?);
                    if let Some(FnArg::Typed(env_pt)) = args.pop_front() {
                        if !is_env_param(&env_pt) {
                            params.push(make_jni_param(env_pt)?);
                        }
                    }
                    for arg in args {
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

    if !consumed_env {
        if let Some(arg) = args.pop_front() {
            if let FnArg::Typed(pt) = arg {
                if !is_env_param(&pt) {
                    return Err(Error::new_spanned(
                        pt.ty,
                        "second parameter must be `env: &mut jni::JNIEnv`",
                    ));
                }
            }
        }
    }

    if let Some(arg) = args.front() {
        if let FnArg::Typed(pt) = arg {
            if is_this_param(pt) {
                let FnArg::Typed(pt) = args.pop_front().unwrap() else {
                    unreachable!()
                };
                params.push(make_jni_param(pt)?);
            }
        }
    }

    if matches!(kind, MethodKind::Override { .. }) {
        if let Some(arg) = args.pop_front() {
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

    for arg in args {
        match arg {
            FnArg::Typed(pt) => params.push(make_jni_param(pt)?),
            FnArg::Receiver(receiver) => {
                return Err(Error::new_spanned(
                    receiver,
                    "unexpected receiver in JNI param list",
                ))
            }
        }
    }

    Ok(params)
}

fn is_env_param(pt: &PatType) -> bool {
    if let syn::Pat::Ident(pi) = pt.pat.as_ref() {
        if pi.ident != "env" && pi.ident != "_env" {
            return false;
        }
    } else {
        return false;
    }

    matches!(pt.ty.as_ref(), Type::Reference(_))
}

fn is_super_caller_param(pt: &PatType) -> bool {
    if let syn::Pat::Ident(pi) = pt.pat.as_ref() {
        pi.ident == "super_"
    } else {
        false
    }
}

fn is_this_param(pt: &PatType) -> bool {
    if let syn::Pat::Ident(pi) = pt.pat.as_ref() {
        pi.ident == "this" || pi.ident == "_this"
    } else {
        false
    }
}

fn make_jni_param(pt: PatType) -> Result<JniParam> {
    let name = match pt.pat.as_ref() {
        syn::Pat::Ident(pi) => pi.ident.clone(),
        other => {
            return Err(Error::new_spanned(
                other,
                "JNI parameter must be a simple identifier",
            ))
        }
    };

    let class_attr = extract_class_attr(&pt.attrs)?;
    Ok(JniParam {
        name,
        ty: *pt.ty,
        class_attr,
    })
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
