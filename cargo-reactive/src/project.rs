//! The app being built: its Cargo package plus `[package.metadata.reactive]`.

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command;

/// `[package.metadata.reactive]`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Metadata {
    /// Display name. Defaults to the package name.
    pub name: Option<String>,
    /// Reverse-DNS app ID: Android application ID, iOS/macOS bundle ID, GTK
    /// application ID.
    pub identifier: Option<String>,
    /// Square PNG, relative to the crate directory.
    pub icon: Option<PathBuf>,
    pub android: AndroidMetadata,
    pub ios: IosMetadata,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AndroidMetadata {
    pub min_sdk: Option<u32>,
    pub target_sdk: Option<u32>,
    pub compile_sdk: Option<u32>,
    pub version_code: Option<u32>,
    pub permissions: Vec<String>,
    /// ABIs for release builds. Debug runs build only the device's ABI.
    pub abis: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct IosMetadata {
    pub deployment_target: Option<String>,
    pub development_team: Option<String>,
    pub build_number: Option<String>,
}

pub struct Project {
    /// Cargo package name.
    pub package: String,
    /// Library target name (`-` replaced by `_`), the stem of the built
    /// `.so`/`.a`.
    pub lib_name: String,
    /// The desktop binary, if the package has one.
    pub bin_name: Option<String>,
    pub version: String,
    /// Directory containing the package's `Cargo.toml`.
    pub crate_dir: PathBuf,
    pub target_dir: PathBuf,
    pub name: String,
    pub identifier: String,
    pub icon: Option<PathBuf>,
    pub metadata: Metadata,
    /// Root of a local checkout of the framework, when the app depends on
    /// `reactive` by path. Its Gradle builds are then used directly instead of
    /// published artifacts.
    pub framework_root: Option<PathBuf>,
}

#[derive(Deserialize)]
struct CargoMetadata {
    packages: Vec<Package>,
    target_directory: PathBuf,
}

#[derive(Deserialize)]
struct Package {
    name: String,
    version: String,
    manifest_path: PathBuf,
    targets: Vec<Target>,
    dependencies: Vec<Dependency>,
    metadata: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct Target {
    name: String,
    kind: Vec<String>,
}

#[derive(Deserialize)]
struct Dependency {
    name: String,
    path: Option<PathBuf>,
}

impl Project {
    /// Loads the package selected by `package`, or else the one whose
    /// `Cargo.toml` is nearest to the current directory.
    pub fn load(manifest_path: Option<&Path>, package: Option<&str>) -> Result<Self> {
        let manifest_path = match manifest_path {
            Some(path) => path.canonicalize().context("--manifest-path")?,
            None => locate_manifest()?,
        };

        let output = Command::new(cargo())
            .args([
                "metadata",
                "--no-deps",
                "--format-version",
                "1",
                "--manifest-path",
            ])
            .arg(&manifest_path)
            .output()
            .context("run `cargo metadata`")?;
        if !output.status.success() {
            bail!(
                "`cargo metadata` failed:\n{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let metadata: CargoMetadata =
            serde_json::from_slice(&output.stdout).context("parse `cargo metadata`")?;

        let pkg = match package {
            Some(name) => metadata
                .packages
                .into_iter()
                .find(|p| p.name == name)
                .with_context(|| format!("no package `{name}` in this workspace"))?,
            None => metadata
                .packages
                .into_iter()
                .find(|p| p.manifest_path == manifest_path)
                .with_context(|| {
                    format!(
                        "{} is a workspace manifest; pick the app with `-p <package>`",
                        manifest_path.display()
                    )
                })?,
        };

        let lib = pkg
            .targets
            .iter()
            .find(|t| {
                t.kind
                    .iter()
                    .any(|k| k == "cdylib" || k == "staticlib" || k == "lib")
            })
            .with_context(|| format!("package `{}` has no library target", pkg.name))?;
        let lib_name = lib.name.replace('-', "_");
        let bin_name = pkg
            .targets
            .iter()
            .find(|t| t.kind.iter().any(|k| k == "bin"))
            .map(|t| t.name.clone());

        let meta: Metadata = match pkg.metadata.and_then(|m| m.get("reactive").cloned()) {
            Some(value) => serde_json::from_value(value)
                .context("invalid [package.metadata.reactive] in Cargo.toml")?,
            None => Metadata::default(),
        };

        let crate_dir = pkg.manifest_path.parent().unwrap().to_path_buf();
        let identifier = meta
            .identifier
            .clone()
            .unwrap_or_else(|| format!("com.example.{}", pkg.name.replace('-', "_")));
        validate_identifier(&identifier)?;

        let framework_root = pkg
            .dependencies
            .iter()
            .find(|d| d.name == "reactive")
            .and_then(|d| d.path.as_ref())
            .and_then(|p| p.parent())
            .filter(|root| root.join("android-lib").is_dir())
            .map(Path::to_path_buf);

        Ok(Self {
            name: meta.name.clone().unwrap_or_else(|| pkg.name.clone()),
            identifier,
            icon: meta.icon.as_ref().map(|icon| crate_dir.join(icon)),
            package: pkg.name,
            lib_name,
            bin_name,
            version: pkg.version,
            crate_dir,
            target_dir: metadata.target_directory,
            metadata: meta,
            framework_root,
        })
    }

    /// The platform project directory: `<crate>/<platform>` once ejected,
    /// otherwise `<crate>/gen/<platform>`, which is regenerated on every run.
    pub fn platform_dir(&self, platform: &str) -> (PathBuf, bool) {
        let ejected = self.crate_dir.join(platform);
        if ejected.is_dir() {
            (ejected, true)
        } else {
            (self.crate_dir.join("gen").join(platform), false)
        }
    }

    pub fn icon(&self) -> Result<Option<&Path>> {
        match &self.icon {
            Some(icon) if !icon.is_file() => {
                bail!("icon {} does not exist", icon.display())
            }
            icon => Ok(icon.as_deref()),
        }
    }
}

fn locate_manifest() -> Result<PathBuf> {
    let output = Command::new(cargo())
        .args(["locate-project", "--message-format", "plain"])
        .output()
        .context("run `cargo locate-project`")?;
    if !output.status.success() {
        bail!("{}", String::from_utf8_lossy(&output.stderr).trim());
    }
    Ok(PathBuf::from(String::from_utf8(output.stdout)?.trim()))
}

/// The cargo that invoked us (`cargo reactive`), or the one on `PATH`.
pub fn cargo() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".into())
}

/// Android application IDs are the strictest of the platforms: dot-separated
/// segments of `[A-Za-z][A-Za-z0-9_]*`, at least two of them.
fn validate_identifier(id: &str) -> Result<()> {
    let segments: Vec<_> = id.split('.').collect();
    let valid = segments.len() >= 2
        && segments.iter().all(|s| {
            let mut chars = s.chars();
            chars.next().is_some_and(|c| c.is_ascii_alphabetic())
                && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        });
    if !valid {
        bail!(
            "identifier `{id}` is invalid: use reverse-DNS segments of letters, digits and \
             underscores, each starting with a letter (for example `com.example.my_app`)"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers() {
        assert!(validate_identifier("com.example.my_app").is_ok());
        assert!(validate_identifier("com.example.my-app").is_err());
        assert!(validate_identifier("app").is_err());
        assert!(validate_identifier("com.1example").is_err());
    }
}
