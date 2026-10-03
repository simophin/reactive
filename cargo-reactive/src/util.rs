use anyhow::{Context, Result, bail};
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

/// Replaces every `{{key}}` in `template`. Fails on unknown keys so a typo in
/// a template can't silently ship.
pub fn render(template: &str, vars: &HashMap<&str, String>) -> Result<String> {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let end = rest[start..]
            .find("}}")
            .context("unterminated `{{` in template")?
            + start;
        let key = &rest[start + 2..end];
        let value = vars
            .get(key)
            .with_context(|| format!("template variable `{key}` is not set"))?;
        out.push_str(value);
        rest = &rest[end + 2..];
    }
    out.push_str(rest);
    Ok(out)
}

/// Writes `contents` unless the file already holds exactly that, so
/// regenerating doesn't touch modification times.
pub fn write_if_changed(path: &Path, contents: impl AsRef<[u8]>) -> Result<()> {
    let contents = contents.as_ref();
    if std::fs::read(path).is_ok_and(|existing| existing == contents) {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, contents).with_context(|| format!("write {}", path.display()))
}

#[cfg(unix)]
pub fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
    Ok(())
}

#[cfg(not(unix))]
pub fn make_executable(_path: &Path) -> Result<()> {
    Ok(())
}

/// Path to `to`, relative to the directory `from`. Both must be absolute.
pub fn relative_path(from: &Path, to: &Path) -> PathBuf {
    let from: Vec<_> = from.components().collect();
    let to: Vec<_> = to.components().collect();
    let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let mut path = PathBuf::new();
    for _ in common..from.len() {
        path.push(Component::ParentDir);
    }
    for c in &to[common..] {
        path.push(c);
    }
    if path.as_os_str().is_empty() {
        path.push(".");
    }
    path
}

/// Gradle and Xcode files want forward slashes even on Windows.
pub fn slash_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Runs `cmd`, echoing it first, and fails if it exits unsuccessfully.
pub fn run(cmd: &mut Command) -> Result<()> {
    eprintln!("     Running {}", describe(cmd));
    let status = cmd
        .status()
        .with_context(|| format!("failed to start `{}`", cmd.get_program().to_string_lossy()))?;
    if !status.success() {
        bail!("`{}` failed with {status}", describe(cmd));
    }
    Ok(())
}

/// Runs `cmd` and returns its trimmed stdout.
pub fn output(cmd: &mut Command) -> Result<String> {
    let out = cmd
        .output()
        .with_context(|| format!("failed to start `{}`", cmd.get_program().to_string_lossy()))?;
    if !out.status.success() {
        bail!(
            "`{}` failed: {}",
            describe(cmd),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn describe(cmd: &Command) -> String {
    std::iter::once(cmd.get_program())
        .chain(cmd.get_args())
        .map(|s| s.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn status(verb: &str, message: impl std::fmt::Display) {
    eprintln!("{verb:>12} {message}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_and_rejects_unknown_keys() {
        let vars = HashMap::from([("a", "1".to_string())]);
        assert_eq!(render("x{{a}}y{{a}}", &vars).unwrap(), "x1y1");
        assert!(render("{{b}}", &vars).is_err());
    }

    #[test]
    fn relative_paths() {
        assert_eq!(
            relative_path(Path::new("/a/b/gen/android"), Path::new("/a/b")),
            PathBuf::from("../..")
        );
        assert_eq!(
            relative_path(Path::new("/a/b"), Path::new("/a/c/d")),
            PathBuf::from("../c/d")
        );
        assert_eq!(
            relative_path(Path::new("/a"), Path::new("/a")),
            PathBuf::from(".")
        );
    }
}
