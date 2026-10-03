//! Checks the toolchains each target needs and says how to fix what's
//! missing.

use crate::android;
use crate::apple::which;
use crate::project::cargo;
use std::path::Path;
use std::process::Command;

struct Report {
    problems: usize,
}

impl Report {
    fn check(&mut self, ok: bool, what: &str, fix: &str) {
        if ok {
            eprintln!("  ✓ {what}");
        } else {
            self.problems += 1;
            eprintln!("  ✗ {what}\n      {fix}");
        }
    }
}

fn succeeds(program: impl AsRef<std::ffi::OsStr>, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .output()
        .is_ok_and(|o| o.status.success())
}

fn installed_targets() -> Vec<String> {
    Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

pub fn doctor() -> bool {
    let mut report = Report { problems: 0 };
    let targets = installed_targets();
    let target = |report: &mut Report, t: &str| {
        report.check(
            targets.iter().any(|i| i == t),
            &format!("Rust target {t}"),
            &format!("rustup target add {t}"),
        );
    };

    eprintln!("Desktop");
    if cfg!(target_os = "linux") {
        report.check(
            succeeds("pkg-config", &["--exists", "gtk4"]),
            "GTK 4 development files",
            "install your distribution's gtk4 development package (e.g. libgtk-4-dev, gtk4-devel)",
        );
    } else if cfg!(target_os = "macos") {
        report.check(true, "AppKit (built in)", "");
    } else {
        report.check(
            false,
            "desktop backend",
            "this OS has no desktop backend yet",
        );
    }

    eprintln!("\nAndroid");
    let sdk = android::sdk_dir();
    report.check(
        sdk.as_deref().is_some_and(Path::is_dir),
        "Android SDK (ANDROID_HOME)",
        "install the SDK (e.g. with Android Studio) and set ANDROID_HOME",
    );
    let ndk = std::env::var_os("ANDROID_NDK_HOME")
        .map(Into::into)
        .or_else(|| sdk.as_ref().map(|s| s.join("ndk")))
        .filter(|p: &std::path::PathBuf| {
            p.is_dir() && p.read_dir().is_ok_and(|mut d| d.next().is_some())
        });
    report.check(
        ndk.is_some(),
        "Android NDK",
        "install it with the SDK manager: sdkmanager \"ndk;<version>\"",
    );
    report.check(
        succeeds(cargo(), &["ndk", "--version"]),
        "cargo-ndk",
        "cargo install cargo-ndk",
    );
    report.check(
        succeeds("java", &["-version"]),
        "Java (JDK 17+)",
        "install a JDK, or set JAVA_HOME to Android Studio's bundled one",
    );
    report.check(
        succeeds(android::adb_path(), &["version"]),
        "adb",
        "install the SDK's platform-tools",
    );
    for t in ["aarch64-linux-android", "x86_64-linux-android"] {
        target(&mut report, t);
    }

    eprintln!("\niOS");
    if cfg!(target_os = "macos") {
        report.check(
            succeeds("xcodebuild", &["-version"]),
            "Xcode",
            "install Xcode, then: sudo xcode-select -s /Applications/Xcode.app",
        );
        report.check(
            which("xcodegen").is_some(),
            "XcodeGen",
            "brew install xcodegen",
        );
        for t in ["aarch64-apple-ios", "aarch64-apple-ios-sim"] {
            target(&mut report, t);
        }
    } else {
        eprintln!("  - needs macOS with Xcode");
    }

    eprintln!();
    if report.problems == 0 {
        eprintln!("All good.");
    } else {
        eprintln!("{} problem(s) found.", report.problems);
    }
    report.problems == 0
}
