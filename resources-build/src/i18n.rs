use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;
use std::{fs, io};

use crate::names::{is_valid_locale, to_const_name, to_pascal_case, to_snake_case};

pub fn gen_i18n(strings_path: &Path, _strings_dir: &str) -> io::Result<String> {
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
    let mut all_ids: BTreeMap<String, Vec<String>> = BTreeMap::new();
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

        let locale_variants: Vec<(&String, &String)> = per_locale
            .iter()
            .filter_map(|(locale, messages)| messages.get(msg_id).map(|(value, _)| (locale, value)))
            .collect();

        if locale_variants.is_empty() {
            continue;
        }

        let msg_type = if params.is_empty() { "()" } else { struct_name.as_str() };
        writeln!(
            out,
            "    pub static {const_name}: &::resources::AssetDescriptor<::resources::TranslationData<{msg_type}>> = \
             &::resources::AssetDescriptor {{"
        )
        .unwrap();

        let (default_locale, default_value) = locale_variants[0];
        writeln!(out, "        default_variant: ::resources::AssetVariant {{").unwrap();
        writeln!(
            out,
            "            qualifiers: ::resources::QualifierSet {{ \
             locale: Some({default_locale:?}), density: None, color_scheme: None }},"
        )
        .unwrap();
        writeln!(out, "            value: ::resources::TranslationData::new({default_value:?}),").unwrap();
        writeln!(out, "        }},").unwrap();

        writeln!(out, "        other_variants: ::std::borrow::Cow::Borrowed(&[").unwrap();
        for (locale, value) in &locale_variants[1..] {
            writeln!(out, "            ::resources::AssetVariant {{").unwrap();
            writeln!(
                out,
                "                qualifiers: ::resources::QualifierSet {{ \
                 locale: Some({locale:?}), density: None, color_scheme: None }},"
            )
            .unwrap();
            writeln!(out, "                value: ::resources::TranslationData::new({value:?}),").unwrap();
            writeln!(out, "            }},").unwrap();
        }
        writeln!(out, "        ]),").unwrap();
        writeln!(out, "    }};").unwrap();
        writeln!(out).unwrap();

        if !params.is_empty() {
            writeln!(out, "    pub struct {struct_name} {{").unwrap();
            for param in params {
                let field = to_snake_case(param);
                writeln!(out, "        pub {field}: ::std::string::String,").unwrap();
            }
            writeln!(out, "    }}").unwrap();
            writeln!(out, "    impl ::resources::Message for {struct_name} {{").unwrap();
            writeln!(out, "        fn apply(&self, template: &str) -> ::std::string::String {{").unwrap();
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
