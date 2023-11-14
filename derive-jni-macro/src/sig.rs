use syn::{GenericArgument, PathArguments, Type, TypeParen, TypePath, TypeTuple};

pub fn resolve_type_signature(t: &Type, boxed: bool) -> Option<&'static str> {
    match (t, boxed) {
        // Handle primitive case
        (Type::Path(TypePath { path, qself }), false)
            if matches!(path.segments.last(), Some(last) if last.arguments.is_empty())
                && qself.is_none() =>
        {
            let rust_type = path.segments.last().unwrap().ident.to_string();

            Some(match rust_type.as_str() {
                "bool" => "Z",
                "i8" | "u8" => "B",
                "i16" | "u16" => "S",
                "i32" | "u32" => "I",
                "usize" | "isize" | "i64" | "u64" => "J",
                "f32" => "F",
                "f64" => "D",
                "char" => "C",
                "String" | "str" => "Ljava/lang/String;",
                _ => "Ljava/lang/Object;",
            })
        }

        // Handle primitive unit type
        (Type::Tuple(TypeTuple { elems, .. }), false) if elems.is_empty() => Some("V"),

        // Handle boxed primitive case
        (Type::Path(TypePath { path, qself }), true)
            if matches!(path.segments.last(), Some(last) if last.arguments.is_empty())
                && qself.is_none() =>
        {
            let rust_type = path.segments.last().unwrap().ident.to_string();

            Some(match rust_type.as_str() {
                "bool" => "Ljava/lang/Boolean;",
                "char" => "Ljava/lang/Character;",
                "i8" | "u8" => "Ljava/lang/Byte;",
                "i16" | "u16" => "Ljava/lang/Short;",
                "i32" | "u32" => "Ljava/lang/Integer;",
                "usize" | "isize" | "i64" | "u64" => "Ljava/lang/Long;",
                "f32" => "Ljava/lang/Float;",
                "f64" => "Ljava/lang/Double;",
                "String" | "str" => "Ljava/lang/String;",
                _ => "Ljava/lang/Object;",
            })
        }

        // Handle boxed unit type
        (Type::Tuple(TypeTuple { elems, .. }), true) if elems.is_empty() => {
            Some("Ljava/lang/Void;")
        }

        // Handle generic case
        (Type::Path(TypePath { path, qself }), false)
            if matches!(path.segments.last(), Some(last) if !last.arguments.is_empty())
                && qself.is_none() =>
        {
            let last_segment = path.segments.last().unwrap();
            let rust_type = last_segment.ident.to_string();
            // First non-lifetime argument
            let first_argument_type = match &last_segment.arguments {
                PathArguments::None => panic!("Should not happen"),
                PathArguments::AngleBracketed(p) => p
                    .args
                    .iter()
                    .map_while(|s| match s {
                        GenericArgument::Type(t) => Some(t),
                        _ => None,
                    })
                    .next()
                    .unwrap(),
                PathArguments::Parenthesized(_) => return None,
            };

            Some(match rust_type.as_str() {
                "Vec" | "VecDeque" => "Ljava/util/List;",
                "HashMap" | "BTreeMap" => "Ljava/util/Map;",
                "Option" => match resolve_type_signature(first_argument_type, true) {
                    Some(s) => s,
                    None => "Ljava/lang/Object;",
                },
                _ => "Ljava/lang/Object;",
            })
        }

        // Reference case
        (Type::Reference(reference), b) => match reference.elem.as_ref() {
            Type::Slice(_) => Some("[Ljava/util/List;"),
            _ => resolve_type_signature(reference.elem.as_ref(), b),
        },

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn resolve_works() {
        let t: Type = parse_quote! { () };
        let Type::Path(p) = t else {
            panic!("Expected path");
        };

        let last = p.path.segments.last().map(|s| s.ident.to_string()).unwrap();
        println!("last = {}", last);
    }
}
