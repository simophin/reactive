use crate::util::{render, slash_path, status, write_if_changed};
use anyhow::{Context, Result, bail};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const CARGO_TOML: &str = include_str!("../templates/new/Cargo.toml");
const LIB_RS: &str = include_str!("../templates/new/src/lib.rs");
const MAIN_RS: &str = include_str!("../templates/new/src/main.rs");
const GITIGNORE: &str = include_str!("../templates/new/gitignore");

pub struct Options {
    pub path: PathBuf,
    pub name: Option<String>,
    pub identifier: Option<String>,
    /// Local framework checkout to depend on by path.
    pub reactive_path: Option<PathBuf>,
}

pub fn new(opts: Options) -> Result<()> {
    let dir = std::path::absolute(&opts.path)?;
    if dir.exists() && dir.read_dir()?.next().is_some() {
        bail!("{} already exists and is not empty", dir.display());
    }
    let crate_name = dir
        .file_name()
        .and_then(|n| n.to_str())
        .context("project path has no usable directory name")?
        .to_string();
    if !crate_name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        || !crate_name.starts_with(|c: char| c.is_ascii_alphabetic())
    {
        bail!("`{crate_name}` is not a valid crate name");
    }
    let lib_name = crate_name.replace('-', "_");

    let reactive_dep = match opts.reactive_path.or_else(local_framework) {
        Some(root) => format!(
            "{{ path = \"{}\" }}",
            slash_path(&std::path::absolute(root)?.join("reactive"))
        ),
        None => format!("\"{}\"", env!("CARGO_PKG_VERSION")),
    };

    let mut vars = HashMap::new();
    vars.insert("crate_name", crate_name.clone());
    vars.insert("lib_name", lib_name.clone());
    vars.insert(
        "app_name",
        opts.name.unwrap_or_else(|| title_case(&crate_name)),
    );
    vars.insert(
        "identifier",
        opts.identifier
            .unwrap_or_else(|| format!("com.example.{lib_name}")),
    );
    vars.insert("reactive_dep", reactive_dep);

    write_if_changed(&dir.join("Cargo.toml"), render(CARGO_TOML, &vars)?)?;
    write_if_changed(&dir.join("src/lib.rs"), render(LIB_RS, &vars)?)?;
    write_if_changed(&dir.join("src/main.rs"), render(MAIN_RS, &vars)?)?;
    write_if_changed(&dir.join(".gitignore"), GITIGNORE)?;

    status(
        "Created",
        format!("app `{crate_name}` in {}", dir.display()),
    );
    eprintln!(
        "\n  cd {}\n  cargo reactive run                    # this machine\n  \
         cargo reactive run --target android   # connected device or emulator\n  \
         cargo reactive run --target ios       # simulator (macOS only)\n  \
         cargo reactive doctor                 # check toolchains\n",
        opts.path.display()
    );
    Ok(())
}

/// The framework checkout this CLI was built from, if it still exists. Until
/// the crates are published, new apps depend on it by path.
fn local_framework() -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent()?;
    root.join("reactive/Cargo.toml")
        .is_file()
        .then(|| root.to_path_buf())
}

fn title_case(name: &str) -> String {
    name.split(['-', '_'])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            let first = chars.next().unwrap().to_ascii_uppercase();
            std::iter::once(first).chain(chars).collect::<String>()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    #[test]
    fn title_case() {
        assert_eq!(super::title_case("my-cool_app"), "My Cool App");
    }
}
