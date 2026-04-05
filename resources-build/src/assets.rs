use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;
use std::{fs, io};

use crate::names::{to_const_name, to_mod_name};

/// One variant of an asset (a particular qualifier directory + file path).
struct Variant {
    /// Rust expression for `::resources::QualifierSet { … }`.
    qualifier_expr: String,
    /// Path for `include_bytes!`, relative to `CARGO_MANIFEST_DIR`.
    /// e.g. `"res/xhdpi/icons/close.png"`
    include_path: String,
    /// Image dimensions `(width, height)` if this file is a recognised image format.
    dimensions: Option<(u32, u32)>,
}

/// Recursive module tree built from the flat asset list.
#[derive(Default)]
struct ModTree {
    /// const_name (SCREAMING_SNAKE_CASE) → variants
    assets: BTreeMap<String, Vec<Variant>>,
    /// module_name → subtree
    children: BTreeMap<String, ModTree>,
}

pub fn gen_assets(res_path: &Path, res_dir: &str) -> io::Result<String> {
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
            let relative = path.strip_prefix(qualifier_dir).unwrap();
            let asset_key = relative
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");

            let include_path = format!("{res_dir}/{dir_name}/{asset_key}");
            let dimensions = read_image_dimensions(&path);
            map.entry(asset_key).or_default().push(Variant {
                qualifier_expr: qualifier_expr.to_owned(),
                include_path,
                dimensions,
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
        "{i3}value: {},",
        binary_data_expr(&default.include_path, default.dimensions)
    )
    .unwrap();
    writeln!(out, "{i2}}},").unwrap();

    writeln!(out, "{i2}other_variants: ::std::borrow::Cow::Borrowed(&[").unwrap();
    for v in rest {
        writeln!(out, "{i3}::resources::AssetVariant {{").unwrap();
        writeln!(out, "{i3}    qualifiers: {},", v.qualifier_expr).unwrap();
        writeln!(
            out,
            "{i3}    value: {},",
            binary_data_expr(&v.include_path, v.dimensions)
        )
        .unwrap();
        writeln!(out, "{i3}}},").unwrap();
    }
    writeln!(out, "{i2}]),").unwrap();
    writeln!(out, "{indent}}};").unwrap();
}

/// Build a `::resources::BinaryData` expression for a given asset.
fn binary_data_expr(include_path: &str, dimensions: Option<(u32, u32)>) -> String {
    let bytes =
        format!("include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/{include_path}\"))");
    match dimensions {
        Some((w, h)) => {
            format!("::resources::BinaryData::Image {{ data: {bytes}, width: {w}, height: {h} }}")
        }
        None => format!("::resources::BinaryData::Unknown({bytes})"),
    }
}

/// Try to read the pixel dimensions of a recognised image file.
///
/// Delegates to the `imagesize` crate; returns `None` for unrecognised formats.
fn read_image_dimensions(path: &Path) -> Option<(u32, u32)> {
    let size = imagesize::size(path).ok()?;
    Some((size.width as u32, size.height as u32))
}

/// Parse a qualifier directory name into a Rust `QualifierSet { … }` expression.
///
/// Tokens are scanned left-to-right. Known density and color-scheme keywords
/// are consumed; anything else is collected as locale subtags (e.g. `en`,
/// `US` from `en-US`). This allows combinations like `en-US-hdpi` or
/// `fr-night`. Returns `None` only if no qualifier at all was recognised.
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
