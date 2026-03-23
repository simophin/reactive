//! Build-script helper for the `resources` crate.
//!
//! Call [`generate`] from your crate's `build.rs`:
//!
//! ```rust,no_run
//! fn main() {
//!     resources_build::generate("res", "strings").unwrap();
//! }
//! ```
//!
//! Then expose the generated file at the crate root:
//!
//! ```rust,ignore
//! include!(concat!(env!("OUT_DIR"), "/resources.rs"));
//! ```
//!
//! This produces:
//! - A `pub mod assets { … }` tree of typed `AssetDescriptor` constants.
//! - A `pub mod strings { … }` of message structs implementing `resources::Message`.
//! - A `pub fn build_i18n() -> resources::I18n` that constructs the runtime bundle.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;
use std::{fs, io};

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Generate `$OUT_DIR/resources.rs`.
///
/// `res_dir` and `strings_dir` are paths relative to the crate's manifest
/// directory (i.e. relative to `CARGO_MANIFEST_DIR`).  Either directory may
/// be absent — its section is simply omitted from the output.
pub fn generate(res_dir: &str, strings_dir: &str) -> io::Result<()> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");

    let res_path = Path::new(&manifest_dir).join(res_dir);
    let strings_path = Path::new(&manifest_dir).join(strings_dir);

    // Ask cargo to rerun this script if anything under either directory changes.
    println!("cargo:rerun-if-changed={}", res_path.display());
    println!("cargo:rerun-if-changed={}", strings_path.display());

    let mut output = String::new();

    if res_path.is_dir() {
        writeln!(output, "{}", gen_assets(&res_path, res_dir)?).unwrap();
    }

    if strings_path.is_dir() {
        writeln!(output, "{}", gen_i18n(&strings_path, strings_dir)?).unwrap();
    }

    fs::write(Path::new(&out_dir).join("resources.rs"), output)
}

// ---------------------------------------------------------------------------
// Asset code generation
// ---------------------------------------------------------------------------

/// One variant of an asset (a particular qualifier directory + file path).
struct Variant {
    /// Rust expression for `::resources::QualifierSet { … }`.
    qualifier_expr: String,
    /// Path for `include_bytes!`, relative to `CARGO_MANIFEST_DIR`.
    /// e.g. `"res/xhdpi/icons/close.png"`
    include_path: String,
}

/// Recursive module tree built from the flat asset list.
#[derive(Default)]
struct ModTree {
    /// const_name (SCREAMING_SNAKE_CASE) → variants
    assets: BTreeMap<String, Vec<Variant>>,
    /// module_name → subtree
    children: BTreeMap<String, ModTree>,
}

fn gen_assets(res_path: &Path, res_dir: &str) -> io::Result<String> {
    // Collect all assets: BTreeMap<asset_key, variants>
    let mut map: BTreeMap<String, Vec<Variant>> = BTreeMap::new();

    for entry in fs::read_dir(res_path)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let dir_name = entry.file_name().into_string().unwrap_or_default();
        let Some(qualifier_expr) = parse_qualifier_expr(&dir_name) else {
            continue;
        };
        let qualifier_dir = entry.path();
        collect_asset_files(
            &qualifier_dir,
            &qualifier_dir,
            res_dir,
            &dir_name,
            &qualifier_expr,
            &mut map,
        )?;
    }

    // Build module tree
    let mut root = ModTree::default();
    for (key, variants) in map {
        insert_into_tree(&mut root, &key, variants);
    }

    let mut out = String::new();
    writeln!(out, "pub mod assets {{").unwrap();
    emit_tree(&root, &mut out, 1);
    writeln!(out, "}}").unwrap();

    Ok(out)
}

/// Walk `dir` recursively, collecting files into `map`.
fn collect_asset_files(
    dir: &Path,
    qualifier_dir: &Path,
    res_dir: &str,
    dir_name: &str,
    qualifier_expr: &str,
    map: &mut BTreeMap<String, Vec<Variant>>,
) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let path = entry.path();

        if file_type.is_dir() {
            collect_asset_files(&path, qualifier_dir, res_dir, dir_name, qualifier_expr, map)?;
        } else if file_type.is_file() {
            // asset key = path relative to the qualifier directory, forward-slash separated
            let relative = path.strip_prefix(qualifier_dir).unwrap();
            let asset_key = relative
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");

            // include_bytes! path = res_dir/qualifier_dir_name/asset_key
            let include_path = format!("{res_dir}/{dir_name}/{asset_key}");

            map.entry(asset_key).or_default().push(Variant {
                qualifier_expr: qualifier_expr.to_owned(),
                include_path,
            });
        }
    }
    Ok(())
}

/// Insert an asset into the module tree, splitting `key` on `/`.
fn insert_into_tree(tree: &mut ModTree, key: &str, variants: Vec<Variant>) {
    let parts: Vec<&str> = key.splitn(2, '/').collect();
    match parts.as_slice() {
        [file] => {
            let const_name = to_const_name(
                Path::new(file)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(file),
            );
            tree.assets.entry(const_name).or_default().extend(variants);
        }
        [dir, rest] => {
            let mod_name = to_mod_name(dir);
            insert_into_tree(tree.children.entry(mod_name).or_default(), rest, variants);
        }
        _ => {}
    }
}

fn emit_tree(tree: &ModTree, out: &mut String, depth: usize) {
    let indent = "    ".repeat(depth);
    for (const_name, variants) in &tree.assets {
        emit_asset_const(const_name, variants, out, depth);
    }
    for (mod_name, child) in &tree.children {
        writeln!(out, "{indent}pub mod {mod_name} {{").unwrap();
        emit_tree(child, out, depth + 1);
        writeln!(out, "{indent}}}").unwrap();
    }
}

fn emit_asset_const(name: &str, variants: &[Variant], out: &mut String, depth: usize) {
    let indent = "    ".repeat(depth);
    let i2 = "    ".repeat(depth + 1);
    let i3 = "    ".repeat(depth + 2);

    // First variant becomes default_variant; the rest go into other_variants.
    let (default, rest) = variants
        .split_first()
        .expect("asset must have at least one variant");

    writeln!(
        out,
        "{indent}pub static {name}: &::resources::AssetDescriptor<::resources::BinaryData> = \
         &::resources::AssetDescriptor {{"
    )
    .unwrap();

    writeln!(out, "{i2}default_variant: ::resources::AssetVariant {{").unwrap();
    writeln!(out, "{i3}qualifiers: {},", default.qualifier_expr).unwrap();
    writeln!(
        out,
        "{i3}value: ::resources::BinaryData(include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/{}\"))),",
        default.include_path
    )
    .unwrap();
    writeln!(out, "{i2}}},").unwrap();

    writeln!(out, "{i2}other_variants: ::std::borrow::Cow::Borrowed(&[").unwrap();
    for v in rest {
        writeln!(out, "{i3}::resources::AssetVariant {{").unwrap();
        writeln!(out, "{i3}    qualifiers: {},", v.qualifier_expr).unwrap();
        writeln!(
            out,
            "{i3}    value: ::resources::BinaryData(include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/{}\"))),",
            v.include_path
        )
        .unwrap();
        writeln!(out, "{i3}}},").unwrap();
    }
    writeln!(out, "{i2}]),").unwrap();
    writeln!(out, "{indent}}};").unwrap();
}

/// Parse a qualifier directory name into a Rust `QualifierSet { … }` expression.
///
/// Tokens are scanned left-to-right.  Known density and color-scheme keywords
/// are consumed; anything else is collected as locale subtags (e.g. `en`,
/// `US` from `en-US`).  This allows combinations like `en-US-hdpi` or
/// `fr-night`.  Returns `None` only if no qualifier at all was recognised.
fn parse_qualifier_expr(dir_name: &str) -> Option<String> {
    if dir_name == "default" {
        return Some(
            "::resources::QualifierSet { locale: None, density: None, color_scheme: None }".into(),
        );
    }

    let mut locale_parts: Vec<&str> = Vec::new();
    let mut density = "None".to_string();
    let mut color_scheme = "None".to_string();
    let mut matched = false;

    for token in dir_name.split('-') {
        match token {
            "ldpi" => {
                density = "Some(::resources::Density::Ldpi)".into();
                matched = true;
            }
            "mdpi" => {
                density = "Some(::resources::Density::Mdpi)".into();
                matched = true;
            }
            "hdpi" => {
                density = "Some(::resources::Density::Hdpi)".into();
                matched = true;
            }
            "xhdpi" => {
                density = "Some(::resources::Density::Xhdpi)".into();
                matched = true;
            }
            "xxhdpi" => {
                density = "Some(::resources::Density::Xxhdpi)".into();
                matched = true;
            }
            "xxxhdpi" => {
                density = "Some(::resources::Density::Xxxhdpi)".into();
                matched = true;
            }
            "night" => {
                color_scheme = "Some(::resources::ColorScheme::Dark)".into();
                matched = true;
            }
            t => {
                // Accumulate as a locale subtag.  Validate that the first
                // subtag looks like a BCP-47 language tag (2–3 lowercase
                // ASCII letters) so we don't silently accept typos.
                if locale_parts.is_empty() {
                    let is_lang =
                        t.len() >= 2 && t.len() <= 3 && t.chars().all(|c| c.is_ascii_lowercase());
                    if !is_lang {
                        return None;
                    }
                }
                locale_parts.push(t);
                matched = true;
            }
        }
    }

    if !matched {
        return None;
    }

    let locale = if locale_parts.is_empty() {
        "None".to_string()
    } else {
        let locale_str = locale_parts.join("-");
        format!("Some({locale_str:?})")
    };

    Some(format!(
        "::resources::QualifierSet {{ locale: {locale}, density: {density}, color_scheme: {color_scheme} }}"
    ))
}

// ---------------------------------------------------------------------------
// i18n code generation
// ---------------------------------------------------------------------------

fn gen_i18n(strings_path: &Path, _strings_dir: &str) -> io::Result<String> {
    // locale → (message_id → (value_text, param_names))
    let mut per_locale: BTreeMap<String, BTreeMap<String, (String, Vec<String>)>> = BTreeMap::new();

    // Flat layout: strings/en-US.ftl
    // Directory layout: strings/en-US/*.ftl (all files merged into one locale)
    for entry in fs::read_dir(strings_path)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;

        if file_type.is_file() {
            if path.extension().and_then(|e| e.to_str()) != Some("ftl") {
                continue;
            }
            let locale = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_owned();
            if !is_valid_locale(&locale) {
                continue;
            }
            let source = fs::read_to_string(&path)?;
            merge_locale_messages(per_locale.entry(locale).or_default(), &source);
        } else if file_type.is_dir() {
            let locale = entry.file_name().into_string().unwrap_or_default();
            if !is_valid_locale(&locale) {
                continue;
            }
            for ftl_entry in fs::read_dir(&path)? {
                let ftl_entry = ftl_entry?;
                if ftl_entry.path().extension().and_then(|e| e.to_str()) != Some("ftl") {
                    continue;
                }
                let source = fs::read_to_string(ftl_entry.path())?;
                merge_locale_messages(per_locale.entry(locale.clone()).or_default(), &source);
            }
        }
    }

    // Union of all message IDs and their params across all locales.
    let mut all_ids: BTreeMap<String, Vec<String>> = BTreeMap::new(); // id → params
    for messages in per_locale.values() {
        for (id, (_, params)) in messages {
            let entry = all_ids.entry(id.clone()).or_default();
            for p in params {
                if !entry.contains(p) {
                    entry.push(p.clone());
                }
            }
        }
    }

    let mut out = String::new();
    writeln!(out, "pub mod strings {{").unwrap();

    for (msg_id, params) in &all_ids {
        let const_name = to_const_name(msg_id);
        let struct_name = to_pascal_case(msg_id);

        // --- TranslationDescriptor const ---
        // Collect variants in locale order; the first becomes default_variant.
        let locale_variants: Vec<(&String, &String)> = per_locale
            .iter()
            .filter_map(|(locale, messages)| messages.get(msg_id).map(|(value, _)| (locale, value)))
            .collect();

        // Skip messages that have no translations at all (shouldn't happen, but be safe).
        if locale_variants.is_empty() {
            continue;
        }

        // The const uses ::new() so the private PhantomData field stays hidden.
        // No-param messages use () — no struct needed.
        let msg_type = if params.is_empty() {
            "()"
        } else {
            struct_name.as_str()
        };
        writeln!(
            out,
            "    pub static {const_name}: &::resources::AssetDescriptor<::resources::TranslationData<{msg_type}>> = \
             &::resources::AssetDescriptor {{"
        )
        .unwrap();

        // First locale → default_variant (guaranteed fallback).
        let (default_locale, default_value) = locale_variants[0];
        writeln!(out, "        default_variant: ::resources::AssetVariant {{").unwrap();
        writeln!(
            out,
            "            qualifiers: ::resources::QualifierSet {{ \
             locale: Some({default_locale:?}), density: None, color_scheme: None }},"
        )
        .unwrap();
        writeln!(
            out,
            "            value: ::resources::TranslationData::new({default_value:?}),"
        )
        .unwrap();
        writeln!(out, "        }},").unwrap();

        writeln!(
            out,
            "        other_variants: ::std::borrow::Cow::Borrowed(&["
        )
        .unwrap();
        for (locale, value) in &locale_variants[1..] {
            writeln!(out, "            ::resources::AssetVariant {{").unwrap();
            writeln!(
                out,
                "                qualifiers: ::resources::QualifierSet {{ \
                 locale: Some({locale:?}), density: None, color_scheme: None }},"
            )
            .unwrap();
            writeln!(
                out,
                "                value: ::resources::TranslationData::new({value:?}),"
            )
            .unwrap();
            writeln!(out, "            }},").unwrap();
        }
        writeln!(out, "        ]),").unwrap();
        writeln!(out, "    }};").unwrap();
        writeln!(out).unwrap();

        // --- Message struct (only for parameterized messages) ---
        if !params.is_empty() {
            writeln!(out, "    pub struct {struct_name} {{").unwrap();
            for param in params {
                let field = to_snake_case(param);
                writeln!(out, "        pub {field}: ::std::string::String,").unwrap();
            }
            writeln!(out, "    }}").unwrap();
            writeln!(out, "    impl ::resources::Message for {struct_name} {{").unwrap();
            writeln!(
                out,
                "        fn apply(&self, template: &str) -> ::std::string::String {{"
            )
            .unwrap();
            writeln!(out, "            let s = template.to_owned();").unwrap();
            for param in params {
                let field = to_snake_case(param);
                writeln!(
                    out,
                    "            let s = ::resources::replace_param(&s, {param:?}, &self.{field});"
                )
                .unwrap();
            }
            writeln!(out, "            s").unwrap();
            writeln!(out, "        }}").unwrap();
            writeln!(out, "    }}").unwrap();
        }
        writeln!(out).unwrap();
    }

    writeln!(out, "}}").unwrap();
    Ok(out)
}

/// Parse a `.ftl` source and merge messages into `map`.
/// Each entry: message_id → (value_text, param_names).
/// Multi-file locales (directory layout) are merged by calling this repeatedly.
fn merge_locale_messages(map: &mut BTreeMap<String, (String, Vec<String>)>, source: &str) {
    let mut current_id: Option<String> = None;
    let mut current_value = String::new();
    let mut current_params: Vec<String> = Vec::new();

    let flush = |map: &mut BTreeMap<String, (String, Vec<String>)>,
                 id: String,
                 value: String,
                 params: Vec<String>| {
        let entry = map.entry(id).or_insert_with(|| (value.clone(), Vec::new()));
        // Merge params (union across files)
        for p in params {
            if !entry.1.contains(&p) {
                entry.1.push(p);
            }
        }
    };

    for line in source.lines() {
        if let Some((id, value_start)) = parse_message_id_and_value(line) {
            if let Some(prev_id) = current_id.take() {
                flush(
                    map,
                    prev_id,
                    current_value.trim().to_owned(),
                    current_params.drain(..).collect(),
                );
            }
            current_id = Some(id);
            current_value = value_start.to_owned();
            collect_params(line, &mut current_params);
        } else if current_id.is_some() && line.starts_with(' ') {
            // Continuation / attribute line
            current_value.push(' ');
            current_value.push_str(line.trim());
            collect_params(line, &mut current_params);
        }
    }
    if let Some(id) = current_id {
        flush(map, id, current_value.trim().to_owned(), current_params);
    }
}

/// Parse `"message-id = value text"` → `Some(("message-id", "value text"))`.
fn parse_message_id_and_value(line: &str) -> Option<(String, &str)> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with(|c: char| c.is_ascii_alphabetic()) {
        return None;
    }
    let id_end = trimmed.find(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')?;
    let id = &trimmed[..id_end];
    let rest = trimmed[id_end..].trim_start();
    let value = rest.strip_prefix('=')?.trim_start();
    Some((id.to_owned(), value))
}

fn collect_params(text: &str, params: &mut Vec<String>) {
    let mut s = text;
    while let Some(pos) = s.find("{ $").or_else(|| s.find("{$")) {
        let skip = if s[pos..].starts_with("{ $") { 3 } else { 2 };
        s = &s[pos + skip..];
        let end = s
            .find(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
            .unwrap_or(s.len());
        let name = s[..end].to_owned();
        if !name.is_empty() && !params.contains(&name) {
            params.push(name);
        }
    }
}

// ---------------------------------------------------------------------------
// Naming helpers
// ---------------------------------------------------------------------------

/// `close`, `hero-image` → `CLOSE`, `HERO_IMAGE`
fn to_const_name(stem: &str) -> String {
    let s = stem.replace('-', "_").to_uppercase();
    if s.starts_with(|c: char| c.is_ascii_digit()) {
        format!("_{s}")
    } else {
        s
    }
}

/// `icons`, `hero-images` → `icons`, `hero_images`
fn to_mod_name(dir: &str) -> String {
    let s = dir.replace('-', "_").to_lowercase();
    if s.starts_with(|c: char| c.is_ascii_digit()) {
        format!("_{s}")
    } else {
        s
    }
}

/// `unread-messages`, `greeting` → `UnreadMessages`, `Greeting`
fn to_pascal_case(s: &str) -> String {
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
fn to_snake_case(s: &str) -> String {
    s.replace('-', "_")
}

/// Rough sanity check that a string could be a BCP-47 locale identifier.
fn is_valid_locale(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}
