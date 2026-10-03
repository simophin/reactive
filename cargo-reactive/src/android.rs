//! Android: a Gradle project that applies `com.reactive.android`, which builds
//! the crate with cargo-ndk.

use crate::project::Project;
use crate::util::{self, output, render, run, slash_path, status, write_if_changed};
use anyhow::{Context, Result, bail};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

const AGP_VERSION: &str = "9.3.3";
const REACTIVE_VERSION: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_MIN_SDK: u32 = 26;
const DEFAULT_SDK: u32 = 36;
const DEFAULT_RELEASE_ABIS: &[&str] = &["arm64-v8a", "x86_64"];
const ACTIVITY: &str = "com.reactive.ReactiveActivity";

const SETTINGS: &str = include_str!("../templates/android/settings.gradle.kts");
const BUILD: &str = include_str!("../templates/android/build.gradle.kts");
const PROPERTIES: &str = include_str!("../templates/android/gradle.properties");
const MANIFEST: &str = include_str!("../templates/android/AndroidManifest.xml");
const GRADLEW: &[u8] = include_bytes!("../templates/android/gradlew");
const GRADLEW_BAT: &[u8] = include_bytes!("../templates/android/gradlew.bat");
const WRAPPER_JAR: &[u8] = include_bytes!("../templates/android/gradle/wrapper/gradle-wrapper.jar");
const WRAPPER_PROPERTIES: &[u8] =
    include_bytes!("../templates/android/gradle/wrapper/gradle-wrapper.properties");

pub struct Options {
    pub release: bool,
    /// Use the published Gradle plugin and support library even when the
    /// framework is a local checkout.
    pub use_maven: bool,
    pub device: Option<String>,
}

/// Returns the Gradle project directory, generating it unless ejected.
pub fn prepare(project: &Project, opts: &Options) -> Result<PathBuf> {
    let (dir, ejected) = project.platform_dir("android");
    if !ejected {
        generate(project, &dir, opts.use_maven)?;
    }
    Ok(dir)
}

pub fn generate(project: &Project, dir: &Path, use_maven: bool) -> Result<()> {
    status("Generating", dir.display());
    let android = &project.metadata.android;
    let mut vars = HashMap::new();
    vars.insert("crate_name", project.package.clone());
    vars.insert("package", project.package.clone());
    vars.insert("lib_name", project.lib_name.clone());
    vars.insert("app_name", xml_escape(&project.name));
    vars.insert("identifier", project.identifier.clone());
    vars.insert("version", project.version.clone());
    vars.insert(
        "version_code",
        android
            .version_code
            .unwrap_or_else(|| version_code(&project.version))
            .to_string(),
    );
    vars.insert(
        "min_sdk",
        android.min_sdk.unwrap_or(DEFAULT_MIN_SDK).to_string(),
    );
    vars.insert(
        "target_sdk",
        android.target_sdk.unwrap_or(DEFAULT_SDK).to_string(),
    );
    vars.insert(
        "compile_sdk",
        android.compile_sdk.unwrap_or(DEFAULT_SDK).to_string(),
    );
    vars.insert("agp_version", AGP_VERSION.into());
    vars.insert("reactive_version", REACTIVE_VERSION.into());
    vars.insert(
        "crate_dir",
        slash_path(&util::relative_path(&absolute(dir)?, &project.crate_dir)),
    );

    let abs_dir = absolute(dir)?;
    // Relative inside the framework checkout (its examples), so committed
    // projects stay portable; absolute for apps elsewhere.
    let rel = |p: PathBuf| match &project.framework_root {
        Some(root) if abs_dir.starts_with(root) => slash_path(&util::relative_path(&abs_dir, &p)),
        _ => slash_path(&p),
    };
    let (plugin_include, lib_include) = match &project.framework_root {
        Some(root) if !use_maven => (
            format!(
                "    // Local framework checkout: build the plugin from source.\n    includeBuild(\"{}\")\n",
                rel(root.join("reactive-gradle-plugin"))
            ),
            format!(
                "\n// Local framework checkout: build the support library from source.\nincludeBuild(\"{}\")\n",
                rel(root.join("android-lib"))
            ),
        ),
        _ => (String::new(), String::new()),
    };
    vars.insert("plugin_include_build", plugin_include);
    vars.insert("lib_include_build", lib_include);

    vars.insert(
        "permissions",
        android
            .permissions
            .iter()
            .map(|p| {
                format!(
                    "    <uses-permission android:name=\"{}\" />\n",
                    xml_escape(p)
                )
            })
            .collect(),
    );

    let res = dir.join("src/main/res");
    let icon_path = res.join("mipmap-xxxhdpi/ic_launcher.png");
    match project.icon()? {
        Some(icon) => {
            write_if_changed(&icon_path, std::fs::read(icon)?)?;
            vars.insert(
                "icon_attr",
                "\n        android:icon=\"@mipmap/ic_launcher\"".into(),
            );
        }
        None => {
            let _ = std::fs::remove_file(&icon_path);
            vars.insert("icon_attr", String::new());
        }
    }

    write_if_changed(&dir.join("settings.gradle.kts"), render(SETTINGS, &vars)?)?;
    write_if_changed(&dir.join("build.gradle.kts"), render(BUILD, &vars)?)?;
    write_if_changed(&dir.join("gradle.properties"), PROPERTIES)?;
    write_if_changed(
        &dir.join("src/main/AndroidManifest.xml"),
        render(MANIFEST, &vars)?,
    )?;
    write_if_changed(&dir.join("gradlew"), GRADLEW)?;
    util::make_executable(&dir.join("gradlew"))?;
    write_if_changed(&dir.join("gradlew.bat"), GRADLEW_BAT)?;
    write_if_changed(&dir.join("gradle/wrapper/gradle-wrapper.jar"), WRAPPER_JAR)?;
    write_if_changed(
        &dir.join("gradle/wrapper/gradle-wrapper.properties"),
        WRAPPER_PROPERTIES,
    )?;
    Ok(())
}

pub fn build(project: &Project, opts: &Options) -> Result<()> {
    let dir = prepare(project, opts)?;
    let abis = if opts.release {
        release_abis(project)
    } else {
        // Without a device, build what an emulator or a modern phone runs.
        DEFAULT_RELEASE_ABIS.join(",")
    };
    let task = if opts.release {
        "assembleRelease"
    } else {
        "assembleDebug"
    };
    gradle(&dir, opts, &[task, &format!("-Preactive.abis={abis}")])?;

    let variant = if opts.release { "release" } else { "debug" };
    let apks = dir.join("build/outputs/apk").join(variant);
    status("Finished", format!("APK in {}", apks.display()));
    if opts.release {
        eprintln!(
            "             The release APK is unsigned. Eject the Android project \
             (`cargo reactive eject android`) to add a signing config."
        );
    }
    Ok(())
}

pub fn run_app(project: &Project, opts: &Options) -> Result<()> {
    let dir = prepare(project, opts)?;
    let serial = pick_device(opts.device.as_deref())?;
    let abi = output(adb(&serial).args(["shell", "getprop", "ro.product.cpu.abi"]))?;
    status("Device", format!("{serial} ({abi})"));

    let abis = if opts.release {
        release_abis(project)
    } else {
        abi
    };
    let task = if opts.release {
        "installRelease"
    } else {
        "installDebug"
    };
    let gradle_opts = Options {
        device: Some(serial.clone()),
        ..*opts
    };
    gradle(
        &dir,
        &gradle_opts,
        &[task, &format!("-Preactive.abis={abis}")],
    )?;

    let component = format!("{}/{ACTIVITY}", project.identifier);
    run(adb(&serial).args(["shell", "am", "start", "-S", "-n", &component]))?;
    status(
        "Launched",
        format!(
            "{}. Logs: adb -s {serial} logcat --pid=$(adb -s {serial} shell pidof {})",
            project.name, project.identifier
        ),
    );
    Ok(())
}

impl Options {
    fn env(&self, cmd: &mut Command) {
        if let Some(serial) = &self.device {
            cmd.env("ANDROID_SERIAL", serial);
        }
    }
}

fn gradle(dir: &Path, opts: &Options, args: &[&str]) -> Result<()> {
    let wrapper = if cfg!(windows) {
        "gradlew.bat"
    } else {
        "./gradlew"
    };
    let mut cmd = Command::new(if cfg!(windows) {
        dir.join(wrapper)
    } else {
        wrapper.into()
    });
    cmd.current_dir(dir).args(args);
    opts.env(&mut cmd);
    run(&mut cmd)
}

fn release_abis(project: &Project) -> String {
    let abis = &project.metadata.android.abis;
    if abis.is_empty() {
        DEFAULT_RELEASE_ABIS.join(",")
    } else {
        abis.join(",")
    }
}

fn adb(serial: &str) -> Command {
    let mut cmd = Command::new(adb_path());
    cmd.args(["-s", serial]);
    cmd
}

pub fn adb_path() -> PathBuf {
    sdk_dir()
        .map(|sdk| {
            sdk.join("platform-tools")
                .join(if cfg!(windows) { "adb.exe" } else { "adb" })
        })
        .filter(|p| p.is_file())
        .unwrap_or_else(|| "adb".into())
}

pub fn sdk_dir() -> Option<PathBuf> {
    ["ANDROID_HOME", "ANDROID_SDK_ROOT"]
        .iter()
        .find_map(std::env::var_os)
        .map(PathBuf::from)
}

/// The requested device, or the only connected one.
fn pick_device(requested: Option<&str>) -> Result<String> {
    if let Some(serial) = requested {
        return Ok(serial.into());
    }
    let list = output(Command::new(adb_path()).arg("devices"))
        .context("list Android devices (is the SDK's platform-tools installed?)")?;
    let devices: Vec<_> = list
        .lines()
        .skip(1)
        .filter_map(|line| line.split_once('\t'))
        .filter(|(_, state)| *state == "device")
        .map(|(serial, _)| serial.to_string())
        .collect();
    match devices.as_slice() {
        [] => bail!("no Android device connected; start an emulator or plug in a phone"),
        [one] => Ok(one.clone()),
        many => bail!(
            "several Android devices connected ({}); pick one with --device",
            many.join(", ")
        ),
    }
}

/// `major.minor.patch` → `major * 1_000_000 + minor * 1_000 + patch`, so
/// version codes grow with the crate version.
fn version_code(version: &str) -> u32 {
    let mut parts = version
        .split(['.', '-', '+'])
        .map(|p| p.parse::<u32>().unwrap_or(0));
    let (major, minor, patch) = (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    );
    (major * 1_000_000 + minor.min(999) * 1_000 + patch.min(999)).max(1)
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn absolute(path: &Path) -> Result<PathBuf> {
    Ok(std::path::absolute(path)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_codes() {
        assert_eq!(version_code("0.1.0"), 1_000);
        assert_eq!(version_code("1.2.3"), 1_002_003);
        assert_eq!(version_code("0.0.0"), 1);
        assert_eq!(version_code("2.0.1-beta.1"), 2_000_001);
    }
}
