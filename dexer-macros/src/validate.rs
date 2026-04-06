use proc_macro2::Span;
use syn::{Error, Result, Type};

use crate::parse::{DexClass, DexMethod, JniParam, MethodKind};

/// Valid JNI primitive / object types the macro accepts in parameter position.
const VALID_PRIMITIVES: &[&str] = &[
    "jni :: sys :: jboolean",
    "jni :: sys :: jbyte",
    "jni :: sys :: jchar",
    "jni :: sys :: jshort",
    "jni :: sys :: jint",
    "jni :: sys :: jlong",
    "jni :: sys :: jfloat",
    "jni :: sys :: jdouble",
    // Allow bare aliases too
    "jboolean", "jbyte", "jchar", "jshort", "jint", "jlong", "jfloat", "jdouble",
];

/// Valid return types (in addition to `()` / no annotation for void).
const VALID_RETURNS: &[&str] = &[
    "jni :: objects :: JObject",
    "jni :: sys :: jboolean",
    "jni :: sys :: jbyte",
    "jni :: sys :: jchar",
    "jni :: sys :: jshort",
    "jni :: sys :: jint",
    "jni :: sys :: jlong",
    "jni :: sys :: jfloat",
    "jni :: sys :: jdouble",
    "JObject",
    "jboolean", "jbyte", "jchar", "jshort", "jint", "jlong", "jfloat", "jdouble",
];

pub fn validate(class: &DexClass) -> Result<()> {
    let mut errors: Vec<Error> = Vec::new();

    for method in &class.methods {
        if let Err(e) = validate_method(method) {
            errors.push(e);
        }
    }

    // Exactly one constructor
    let ctor_count = class.methods.iter().filter(|m| matches!(m.kind, MethodKind::Constructor)).count();
    if ctor_count != 1 {
        errors.push(Error::new(
            Span::call_site(),
            format!("expected exactly one #[constructor], found {ctor_count}"),
        ));
    }

    combine_errors(errors)
}

fn validate_method(method: &DexMethod) -> Result<()> {
    let mut errors: Vec<Error> = Vec::new();

    // Validate constructor returns Self
    if let MethodKind::Constructor = &method.kind {
        match &method.return_ty {
            syn::ReturnType::Type(_, ty) => {
                let s = type_to_string(ty);
                if s != "Self" {
                    errors.push(Error::new_spanned(
                        ty,
                        "#[constructor] must return `Self`",
                    ));
                }
            }
            syn::ReturnType::Default => {
                errors.push(Error::new(
                    Span::call_site(),
                    "#[constructor] must return `Self`",
                ));
            }
        }
    }

    // Validate each JNI parameter type
    for param in &method.jni_params {
        if let Err(e) = validate_param(param, &method.kind) {
            errors.push(e);
        }
    }

    // Validate return type for non-constructors
    if !matches!(method.kind, MethodKind::Constructor) {
        if let syn::ReturnType::Type(_, ty) = &method.return_ty {
            if let Err(e) = validate_return_type(ty) {
                errors.push(e);
            }
        }
    }

    combine_errors(errors)
}

fn validate_param(param: &JniParam, kind: &MethodKind) -> Result<()> {
    let ty_str = type_to_string(&param.ty);

    // JObject is allowed but requires #[class] for constructors and overrides
    let is_jobject = ty_str == "jni :: objects :: JObject" || ty_str == "JObject";
    if is_jobject {
        let needs_class = matches!(kind, MethodKind::Constructor | MethodKind::Override { .. });
        if needs_class && param.class_attr.is_none() {
            return Err(Error::new_spanned(
                &param.ty,
                format!(
                    "parameter `{}` has type JObject in a {} method — add #[class = \"your/Class\"] to specify the exact Java type",
                    param.name,
                    match kind {
                        MethodKind::Constructor => "#[constructor]",
                        MethodKind::Override { .. } => "#[override]",
                        _ => unreachable!(),
                    }
                ),
            ));
        }
        return Ok(());
    }

    // Must be a known primitive JNI type
    let is_primitive = VALID_PRIMITIVES.iter().any(|p| {
        ty_str == *p || ty_str.ends_with(&format!("::{}", p.split("::").last().unwrap_or("")))
    });
    if !is_primitive {
        return Err(Error::new_spanned(
            &param.ty,
            format!(
                "unsupported JNI parameter type `{ty_str}`. \
                 Use jni::sys::jint, jni::sys::jlong, jni::sys::jboolean, \
                 jni::sys::jfloat, jni::sys::jdouble, or jni::objects::JObject (with #[class])"
            ),
        ));
    }

    Ok(())
}

fn validate_return_type(ty: &Type) -> Result<()> {
    let ty_str = type_to_string(ty);
    let valid = VALID_RETURNS.iter().any(|r| {
        ty_str == *r
            || ty_str.ends_with(&format!("::{}", r.split("::").last().unwrap_or("")))
    });
    if !valid {
        return Err(Error::new_spanned(
            ty,
            format!(
                "unsupported JNI return type `{ty_str}`. \
                 Use (), jni::objects::JObject, or a jni::sys primitive type"
            ),
        ));
    }
    Ok(())
}

// ─────────────────────────── helpers ─────────────────────────────────────

/// Stringify a Type into a normalised path string for comparison.
pub fn type_to_string(ty: &Type) -> String {
    use quote::ToTokens;
    ty.to_token_stream()
        .to_string()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn combine_errors(errors: Vec<Error>) -> Result<()> {
    let mut iter = errors.into_iter();
    match iter.next() {
        None => Ok(()),
        Some(mut first) => {
            for e in iter { first.combine(e); }
            Err(first)
        }
    }
}
