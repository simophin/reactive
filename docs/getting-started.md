# Getting Started

An app is one Rust library crate. `cargo reactive` builds it for desktop, Android and iOS. The Gradle and Xcode projects are generated for you and kept out of the way.

## Install

```bash
cargo install --path cargo-reactive     # from a checkout of this repository
cargo reactive doctor                   # checks toolchains and says how to fix what's missing
```

Until the crates and Android artifacts are published, apps depend on the framework checkout by path. `cargo reactive new` sets that up when the CLI is installed from a checkout.

## Create and run

```bash
cargo reactive new my-app
cd my-app
cargo reactive run                    # this machine: GTK on Linux, AppKit on macOS
cargo reactive run --target android   # connected device or emulator (--device <serial> to pick)
cargo reactive run --target ios       # simulator, macOS only (--device <name|udid> to pick)
cargo reactive build --target android --release
```

A new app looks like this:

```
my-app/
  Cargo.toml     # crate-type = ["rlib", "cdylib", "staticlib"] + [package.metadata.reactive]
  src/lib.rs     # the app
  src/main.rs    # desktop entry: fn main() { my_app::run() }
  gen/           # generated platform projects (gitignored)
```

```rust
use reactive::prelude::*;

reactive::app!(app);

fn app(ctx: &mut SetupContext) {
    ctx.child(Window::new("My App", |ctx: &mut SetupContext| {
        ctx.child(Label::new("Hello"));
    }, 400.0, 300.0));
}
```

## The `reactive` crate

- **Backend from the target.** macOS uses AppKit, Linux GTK, iOS UIKit and Android the View system. The `gtk` feature selects GTK on macOS.
- **`reactive::prelude`.** It holds the runtime types plus the current platform's concrete widget types (`Window`, `Flex`, `Label`, …), so app code needn't be generic over `Platform`. Reusable components can still be written against `reactive::widgets::Platform`. Native views are under `reactive::platform`.
- **`reactive::app!(setup)`.** This generates:
  - `pub fn run()`, which desktop `main` calls
  - `JNI_OnLoad` on Android
  - `reactive_main` on iOS, which the generated Xcode `main.m` calls
- **The `tokio` feature.** It enters a multi-threaded tokio runtime on the main thread before setup, so timers and IO work inside resources.

## Configuration

Everything platform-specific lives in `Cargo.toml`:

```toml
[package.metadata.reactive]
name = "My App"                     # display name (default: package name)
identifier = "com.example.my_app"   # Android application ID, bundle ID, GTK app ID
icon = "assets/icon.png"            # square PNG, 1024x1024

[package.metadata.reactive.android]
min_sdk = 26
target_sdk = 36
compile_sdk = 36
version_code = 1                    # default: derived from the crate version
permissions = ["android.permission.INTERNET"]
abis = ["arm64-v8a", "x86_64"]      # release builds; debug runs build only the device's ABI

[package.metadata.reactive.ios]
deployment_target = "16.0"
development_team = "ABCDE12345"     # needed for device builds
build_number = "1"
```

## Platform projects

`gen/android` and `gen/ios` are regenerated on every run, so editing them is pointless. When you need something the metadata doesn't cover, such as a signing config, extra Gradle dependencies or an entitlement, take ownership of the project:

```bash
cargo reactive eject android    # moves it to ./android, never regenerated again
cargo reactive eject ios        # moves it to ./ios
```

`cargo reactive generate <platform>` regenerates a project without building it. `cargo reactive open <platform>` opens it in Android Studio or Xcode.

**Android.** The generated Gradle project applies the `com.reactive.android` plugin (`reactive-gradle-plugin/`). The plugin:
- builds the crate with `cargo ndk` for each variant
- packages the `.so` files
- adds the `com.reactive:android-lib` dependency

Existing Android apps can apply the plugin directly:

```kotlin
plugins { id("com.reactive.android") version "0.1.0" }
reactive {
    crateDir.set(file("../rust"))
    packageName.set("my-app")
}
```

**Where the plugin comes from.** When the app depends on a framework checkout, the generated project uses the plugin and library from source through `includeBuild`. With `--use-maven`, or once the framework is published, it resolves them from Maven repositories, `mavenLocal()` first. To publish locally:

```bash
cargo-reactive/templates/android/gradlew -p android-lib publishToMavenLocal
cargo-reactive/templates/android/gradlew -p reactive-gradle-plugin publishToMavenLocal
```

**iOS.** The generated project is an [XcodeGen](https://github.com/yonaskolb/XcodeGen) spec (`brew install xcodegen`):
- A pre-build phase runs `cargo build --lib` for each architecture Xcode asks for and merges the results with `lipo`.
- `main.m` calls `reactive_main`.
- The Info.plist declares the scene manifest that the UIKit backend's `ReactiveSceneDelegate` needs.

**macOS.** `cargo reactive run` wraps the binary in `target/reactive/macos/<Name>.app`, so the app gets a proper bundle identity, name and icon.
