/// `close`, `hero-image` → `CLOSE`, `HERO_IMAGE`
pub fn to_const_name(stem: &str) -> String {
    let s = stem.replace('-', "_").to_uppercase();
    if s.starts_with(|c: char| c.is_ascii_digit()) {
        format!("_{s}")
    } else {
        s
    }
}

/// `icons`, `hero-images` → `icons`, `hero_images`
pub fn to_mod_name(dir: &str) -> String {
    let s = dir.replace('-', "_").to_lowercase();
    if s.starts_with(|c: char| c.is_ascii_digit()) {
        format!("_{s}")
    } else {
        s
    }
}

/// `unread-messages`, `greeting` → `UnreadMessages`, `Greeting`
pub fn to_pascal_case(s: &str) -> String {
    s.split('-')
        .map(|word| {
            let mut c = word.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect()
}

/// `user-name`, `count` → `user_name`, `count`
pub fn to_snake_case(s: &str) -> String {
    s.replace('-', "_")
}

/// Rough sanity check that a string could be a BCP-47 locale identifier.
pub fn is_valid_locale(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}
