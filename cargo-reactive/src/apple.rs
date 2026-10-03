//! macOS: the desktop binary wrapped in an `.app` bundle.
//! iOS: an XcodeGen project whose build phase builds the crate's static
//! library, plus a `main.m` that calls `reactive_main`.

use crate::project::Project;
use crate::util::{self, output, render, run, slash_path, status, write_if_changed};
use anyhow::{Context, Result, bail};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

const PROJECT_YML: &str = include_str!("../templates/ios/project.yml");
const MAIN_M: &str = include_str!("../templates/ios/main.m");
const BUILD_RUST: &str = include_str!("../templates/ios/build-rust.sh");
const DEFAULT_IOS_TARGET: &str = "16.0";

// ---- macOS ----------------------------------------------------------------

/// Builds the desktop binary and wraps it in `target/reactive/macos/<Name>.app`.
pub fn bundle_macos(project: &Project, release: bool) -> Result<PathBuf> {
    let bin = project
        .bin_name
        .as_deref()
        .context("macOS needs a binary target: add src/main.rs calling `<crate>::run()`")?;
    crate::desktop::cargo_build(project, release)?;

    let profile = if release { "release" } else { "debug" };
    let app = project
        .target_dir
        .join("reactive/macos")
        .join(format!("{}.app", project.name));
    let contents = app.join("Contents");
    let exe = contents.join("MacOS").join(bin);
    std::fs::create_dir_all(exe.parent().unwrap())?;
    std::fs::copy(project.target_dir.join(profile).join(bin), &exe)
        .context("copy binary into the app bundle")?;

    let icon = match project.icon()? {
        Some(icon) => {
            make_icns(icon, &contents.join("Resources/AppIcon.icns"))?;
            "  <key>CFBundleIconFile</key><string>AppIcon</string>\n"
        }
        None => "",
    };
    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key><string>{bin}</string>
  <key>CFBundleIdentifier</key><string>{id}</string>
  <key>CFBundleName</key><string>{name}</string>
  <key>CFBundleDisplayName</key><string>{name}</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleVersion</key><string>{version}</string>
  <key>CFBundleShortVersionString</key><string>{version}</string>
  <key>NSHighResolutionCapable</key><true/>
{icon}</dict>
</plist>
"#,
        id = project.identifier,
        name = xml_escape(&project.name),
        version = project.version,
    );
    write_if_changed(&contents.join("Info.plist"), plist)?;
    status("Bundled", app.display());
    Ok(app)
}

pub fn run_macos(project: &Project, release: bool) -> Result<()> {
    let app = bundle_macos(project, release)?;
    // Run the bundled executable directly rather than through `open`, so its
    // output stays in this terminal; it still gets the bundle's identity.
    let exe = app
        .join("Contents/MacOS")
        .join(project.bin_name.as_deref().unwrap());
    run(&mut Command::new(exe))
}

fn make_icns(png: &Path, icns: &Path) -> Result<()> {
    let iconset = icns.with_extension("iconset");
    std::fs::create_dir_all(&iconset)?;
    for size in [16, 32, 128, 256, 512] {
        for scale in [1, 2] {
            let name = if scale == 1 {
                format!("icon_{size}x{size}.png")
            } else {
                format!("icon_{size}x{size}@2x.png")
            };
            let px = (size * scale).to_string();
            output(
                Command::new("sips")
                    .args(["-z", &px, &px])
                    .arg(png)
                    .arg("--out")
                    .arg(iconset.join(name)),
            )?;
        }
    }
    output(
        Command::new("iconutil")
            .args(["-c", "icns", "-o"])
            .arg(icns)
            .arg(&iconset),
    )?;
    std::fs::remove_dir_all(&iconset)?;
    Ok(())
}

// ---- iOS ------------------------------------------------------------------

pub struct IosOptions {
    pub release: bool,
    /// Simulator name or UDID.
    pub device: Option<String>,
}

/// Returns the directory holding the Xcode project, generating it unless
/// ejected.
pub fn prepare_ios(project: &Project) -> Result<PathBuf> {
    let (dir, ejected) = project.platform_dir("ios");
    if !ejected {
        generate_ios(project, &dir)?;
    }
    if dir.join("project.yml").is_file() {
        let xcodegen =
            which("xcodegen").context("XcodeGen is required for iOS: `brew install xcodegen`")?;
        run(Command::new(xcodegen)
            .args(["generate", "--quiet", "--spec", "project.yml"])
            .current_dir(&dir))?;
    }
    Ok(dir)
}

pub fn generate_ios(project: &Project, dir: &Path) -> Result<()> {
    status("Generating", dir.display());
    let ios = &project.metadata.ios;
    let xcode_name = xcode_name(project);
    let mut vars = HashMap::new();
    vars.insert("xcode_name", xcode_name.clone());
    vars.insert("app_name", yaml_escape(&project.name));
    vars.insert("identifier", project.identifier.clone());
    vars.insert("package", project.package.clone());
    vars.insert("lib_name", project.lib_name.clone());
    vars.insert("version", project.version.clone());
    vars.insert(
        "build_number",
        ios.build_number.clone().unwrap_or_else(|| "1".into()),
    );
    vars.insert(
        "deployment_target",
        ios.deployment_target
            .clone()
            .unwrap_or_else(|| DEFAULT_IOS_TARGET.into()),
    );
    vars.insert(
        "development_team",
        ios.development_team.clone().unwrap_or_default(),
    );
    vars.insert("crate_dir_abs", slash_path(&project.crate_dir));
    vars.insert("target_dir", slash_path(&project.target_dir));

    let appicon = dir.join("Assets.xcassets/AppIcon.appiconset");
    match project.icon()? {
        Some(icon) => {
            write_if_changed(&appicon.join("icon.png"), std::fs::read(icon)?)?;
            write_if_changed(
                &appicon.join("Contents.json"),
                r#"{
  "images" : [
    { "filename" : "icon.png", "idiom" : "universal", "platform" : "ios", "size" : "1024x1024" }
  ],
  "info" : { "author" : "cargo-reactive", "version" : 1 }
}
"#,
            )?;
            write_if_changed(
                &dir.join("Assets.xcassets/Contents.json"),
                "{\n  \"info\" : { \"author\" : \"cargo-reactive\", \"version\" : 1 }\n}\n",
            )?;
            vars.insert("icon_source", "      - Assets.xcassets\n".into());
            vars.insert(
                "icon_setting",
                "        ASSETCATALOG_COMPILER_APPICON_NAME: AppIcon\n".into(),
            );
        }
        None => {
            let _ = std::fs::remove_dir_all(dir.join("Assets.xcassets"));
            vars.insert("icon_source", String::new());
            vars.insert("icon_setting", String::new());
        }
    }

    write_if_changed(&dir.join("project.yml"), render(PROJECT_YML, &vars)?)?;
    write_if_changed(&dir.join("main.m"), MAIN_M)?;
    write_if_changed(&dir.join("build-rust.sh"), render(BUILD_RUST, &vars)?)?;
    util::make_executable(&dir.join("build-rust.sh"))?;
    Ok(())
}

pub fn build_ios(project: &Project, opts: &IosOptions) -> Result<()> {
    let dir = prepare_ios(project)?;
    let name = xcode_name(project);
    if project.metadata.ios.development_team.is_none() {
        bail!(
            "building for a device needs a signing team: set `development_team` in \
             [package.metadata.reactive.ios]. Use `cargo reactive run --target ios` for the simulator."
        );
    }
    let config = if opts.release { "Release" } else { "Debug" };
    let derived = derived_data(project);
    run(Command::new("xcodebuild")
        .current_dir(&dir)
        .args(["-project", &format!("{name}.xcodeproj"), "-scheme", &name])
        .args([
            "-configuration",
            config,
            "-destination",
            "generic/platform=iOS",
        ])
        .arg("-derivedDataPath")
        .arg(&derived)
        .args(["-allowProvisioningUpdates", "build"]))?;
    status(
        "Finished",
        derived
            .join(format!("Build/Products/{config}-iphoneos/{name}.app"))
            .display(),
    );
    Ok(())
}

pub fn run_ios(project: &Project, opts: &IosOptions) -> Result<()> {
    let dir = prepare_ios(project)?;
    let name = xcode_name(project);
    let udid = boot_simulator(opts.device.as_deref())?;
    let config = if opts.release { "Release" } else { "Debug" };
    let derived = derived_data(project);
    run(Command::new("xcodebuild")
        .current_dir(&dir)
        .args(["-project", &format!("{name}.xcodeproj"), "-scheme", &name])
        .args(["-configuration", config])
        .args(["-destination", &format!("id={udid}")])
        .arg("-derivedDataPath")
        .arg(&derived)
        .arg("build"))?;

    let app = derived.join(format!(
        "Build/Products/{config}-iphonesimulator/{name}.app"
    ));
    run(Command::new("xcrun")
        .args(["simctl", "install", &udid])
        .arg(&app))?;
    run(Command::new("xcrun").args([
        "simctl",
        "launch",
        "--console-pty",
        "--terminate-running-process",
        &udid,
        &project.identifier,
    ]))
}

pub fn open_ios(project: &Project) -> Result<()> {
    let dir = prepare_ios(project)?;
    run(Command::new("open").arg(dir.join(format!("{}.xcodeproj", xcode_name(project)))))
}

/// Returns the UDID of the requested simulator (name or UDID), the booted one,
/// or the newest available iPhone, booting it if needed.
fn boot_simulator(requested: Option<&str>) -> Result<String> {
    let json = output(Command::new("xcrun").args(["simctl", "list", "devices", "available", "-j"]))
        .context("list simulators (is Xcode installed?)")?;
    let list: serde_json::Value = serde_json::from_str(&json)?;
    let mut devices: Vec<(String, String, String)> = Vec::new(); // (runtime, name, udid)
    let mut booted = None;
    for (runtime, entries) in list["devices"].as_object().into_iter().flatten() {
        if !runtime.contains("iOS") {
            continue;
        }
        for d in entries.as_array().into_iter().flatten() {
            let name = d["name"].as_str().unwrap_or_default().to_string();
            let udid = d["udid"].as_str().unwrap_or_default().to_string();
            if requested.is_some_and(|r| r == name || r == udid) {
                return boot(udid, d["state"].as_str() == Some("Booted"));
            }
            if d["state"].as_str() == Some("Booted") {
                booted.get_or_insert(udid.clone());
            }
            devices.push((runtime.clone(), name, udid));
        }
    }
    if let Some(r) = requested {
        bail!("no available iOS simulator named `{r}`");
    }
    if let Some(udid) = booted {
        return Ok(udid);
    }
    // Runtime keys end in the version (…SimRuntime.iOS-18-2), so the last
    // matching key sorts newest.
    devices.sort();
    let (_, name, udid) = devices
        .into_iter()
        .rev()
        .find(|(_, name, _)| name.starts_with("iPhone"))
        .context("no iPhone simulator available; add one in Xcode")?;
    status("Simulator", &name);
    boot(udid, false)
}

fn boot(udid: String, booted: bool) -> Result<String> {
    if !booted {
        run(Command::new("xcrun").args(["simctl", "boot", &udid]))?;
    }
    let _ = Command::new("open").args(["-a", "Simulator"]).status();
    Ok(udid)
}

fn derived_data(project: &Project) -> PathBuf {
    project.target_dir.join("reactive/ios-derived-data")
}

/// Xcode target and scheme name: the display name without characters that
/// make shell and build-setting quoting painful.
fn xcode_name(project: &Project) -> String {
    let name: String = project
        .name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    if name.is_empty() || name.starts_with(|c: char| c.is_ascii_digit()) {
        project.lib_name.clone()
    } else {
        name
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn yaml_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

pub fn which(program: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(program))
        .find(|p| p.is_file())
}
