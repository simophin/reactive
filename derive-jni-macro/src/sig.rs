use std::borrow::Cow;

use syn::{Type, TypePath};

pub fn resolve_signature(t: &Type) -> Option<Cow<'static, str>> {
    match t {
        Type::Path(TypePath { path, qself }) => {
            if qself.is_some() {
                return None;
            }

            let last_path = path.segments.last().map(|s| s.ident.to_string());

            match (path.segments.len(), last_path.as_ref().map(String::as_str)) {
                (1, Some("bool")) => Some(Cow::Borrowed("Z")),
                (1, Some("u8")) | (1, Some("i8")) => Some(Cow::Borrowed("B")),
                (1, Some("i32")) | (1, Some("u32")) => Some(Cow::Borrowed("I")),
                (1, Some("i64")) | (1, Some("u64")) | (1, Some("isize")) | (1, Some("usize")) => {
                    Some(Cow::Borrowed("J"))
                }
                (1, Some("f32")) => Some(Cow::Borrowed("F")),
                (1, Some("double")) => Some(Cow::Borrowed("D")),
                (1, Some("str")) | (1, Some("String")) => Some(Cow::Borrowed("Ljava/lang/String;")),
                _ => Some(Cow::Borrowed("Ljava/lang/Object;")),
            }
        }

        _ => todo!(),
    }
}
