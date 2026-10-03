//! One crate for writing a reactive native app that builds for every
//! supported platform.
//!
//! The backend is picked from the compilation target:
//!
//! | Target  | Backend                                   |
//! |---------|-------------------------------------------|
//! | macOS   | AppKit (GTK with the `gtk` feature)       |
//! | Linux   | GTK                                       |
//! | iOS     | UIKit                                     |
//! | Android | Android View system                       |
//!
//! An app is a library crate that declares its entry point with [`app!`]:
//!
//! ```ignore
//! use reactive::prelude::*;
//!
//! reactive::app!(app);
//!
//! fn app(ctx: &mut SetupContext) {
//!     ctx.child(Window::new("Hello", |ctx: &mut SetupContext| {
//!         ctx.child(Label::new("Hello, world"));
//!     }, 400.0, 300.0));
//! }
//! ```
//!
//! plus a `src/main.rs` containing `fn main() { my_app::run() }` for desktop.
//! `cargo reactive new` generates both.

/// The reactive runtime: signals, effects, components and control flow.
pub use reactive_core as core;

/// Shared widget traits, modifiers and layout types. Use these to write
/// components that are generic over [`widgets::Platform`].
pub use ui_core::widgets;

/// Platform backends, for native escape hatches.
pub mod platform {
    #[cfg(target_os = "android")]
    pub use ui_core::android;
    #[cfg(all(target_os = "macos", not(feature = "gtk")))]
    pub use ui_core::appkit;
    #[cfg(any(target_os = "linux", all(target_os = "macos", feature = "gtk")))]
    pub use ui_core::gtk;
    #[cfg(target_os = "ios")]
    pub use ui_core::uikit;
}

/// The backend for the current compilation target.
#[cfg(target_os = "android")]
pub type DefaultPlatform = ui_core::android::platform::Android;
/// The backend for the current compilation target.
#[cfg(all(target_os = "macos", not(feature = "gtk")))]
pub type DefaultPlatform = ui_core::appkit::platform::AppKit;
/// The backend for the current compilation target.
#[cfg(any(target_os = "linux", all(target_os = "macos", feature = "gtk")))]
pub type DefaultPlatform = ui_core::gtk::platform::Gtk;
/// The backend for the current compilation target.
#[cfg(target_os = "ios")]
pub type DefaultPlatform = ui_core::uikit::platform::UIKit;

/// Concrete widget types of [`DefaultPlatform`], so app code doesn't have to
/// be generic over the platform.
pub mod ui {
    use super::DefaultPlatform;
    use ui_core::widgets::Platform;

    pub type Button = <DefaultPlatform as Platform>::Button;
    pub type Flex = <DefaultPlatform as Platform>::Flex;
    pub type Image = <DefaultPlatform as Platform>::Image;
    pub type ImageCodec = <DefaultPlatform as Platform>::ImageCodec;
    pub type Label = <DefaultPlatform as Platform>::Label;
    pub type ProgressIndicator = <DefaultPlatform as Platform>::ProgressIndicator;
    pub type Slider = <DefaultPlatform as Platform>::Slider;
    pub type Stack = <DefaultPlatform as Platform>::Stack;
    pub type TextInput = <DefaultPlatform as Platform>::TextInput;
    pub type Window = <DefaultPlatform as Platform>::Window;
}

/// Everything an app usually needs: the reactive runtime, the widget types of
/// the current platform, and the widget traits (imported anonymously so their
/// methods resolve without clashing with the type names).
pub mod prelude {
    pub use crate::DefaultPlatform;
    pub use crate::ui::*;
    pub use reactive_core::{
        BoxedComponent, Component, IntoSignal, Match, ReadStoredSignal, ResourceState,
        SetupContext, Show, Signal, SignalExt, StoredSignal, Switch,
    };
    pub use ui_core::widgets::{
        AlignContent, AlignItems, CommonModifiers, EdgeInsets, FlexDirection, FlexProps, FlexUnit,
        FlexWrap, JustifyContent, Modifier, Platform, TextAlignment, WithModifier,
    };
    pub use ui_core::widgets::{
        Button as _, Flex as _, Image as _, ImageCodec as _, Label as _, ProgressIndicator as _,
        Slider as _, Stack as _, TextInput as _, Window as _,
    };
}

/// Declares the app's entry point. Call it once, in the crate root of the app
/// library, with a `fn(&mut SetupContext)` that builds the component tree.
///
/// It generates:
/// - `pub fn run()`, which starts the platform main loop and blocks until the
///   app exits. Desktop binaries call it from `main`.
/// - On Android, `JNI_OnLoad`, which `ReactiveActivity` reaches by loading the
///   library.
/// - On iOS, `extern "C" fn reactive_main()`, which the Xcode host's `main`
///   calls.
#[macro_export]
macro_rules! app {
    ($setup:expr) => {
        fn __reactive_setup(ctx: &mut $crate::core::SetupContext) {
            $crate::__private::init();
            ($setup)(ctx)
        }

        /// Starts the app and blocks until it exits.
        pub fn run() {
            // `cargo reactive` sets the ID from `[package.metadata.reactive]`.
            const ID: &str = match ::std::option_env!("REACTIVE_APP_ID") {
                Some(id) => id,
                None => ::std::concat!("rs.reactive.", ::std::env!("CARGO_PKG_NAME")),
            };
            $crate::__private::run(ID, __reactive_setup)
        }

        #[cfg(target_os = "android")]
        #[unsafe(no_mangle)]
        pub unsafe extern "system" fn JNI_OnLoad(
            vm: *mut $crate::__private::jni::sys::JavaVM,
            _reserved: *mut ::std::ffi::c_void,
        ) -> $crate::__private::jni::sys::jint {
            unsafe { $crate::__private::android_on_load(vm, __reactive_setup) }
        }

        #[cfg(target_os = "ios")]
        #[unsafe(no_mangle)]
        pub extern "C" fn reactive_main() -> ::std::ffi::c_int {
            run();
            0
        }
    };
}

#[doc(hidden)]
pub mod __private {
    use reactive_core::SetupContext;

    #[cfg(target_os = "android")]
    pub use ui_core::android::__jni as jni;
    #[cfg(target_os = "android")]
    pub use ui_core::android::on_load as android_on_load;

    /// Runs on the main thread before the app's setup function.
    pub fn init() {
        #[cfg(feature = "tokio")]
        enter_tokio();
    }

    /// Enters a process-wide multi-threaded tokio runtime on the calling
    /// thread, for good. Later calls are no-ops.
    #[cfg(feature = "tokio")]
    fn enter_tokio() {
        use std::sync::{LazyLock, Once};

        static RUNTIME: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("build tokio runtime")
        });
        static ENTER: Once = Once::new();
        // The guard must outlive the main loop, which never hands control
        // back on mobile, so it is leaked.
        ENTER.call_once(|| std::mem::forget(RUNTIME.enter()));
    }

    #[cfg(any(target_os = "linux", all(target_os = "macos", feature = "gtk")))]
    pub fn run(app_id: &str, setup: fn(&mut SetupContext)) {
        ui_core::gtk::run_app_with_id(app_id, setup);
    }

    // iOS may disconnect a scene to reclaim memory and reconnect it later, so
    // the UI is rebuilt on every connect.
    #[cfg(target_os = "ios")]
    pub fn run(_app_id: &str, setup: fn(&mut SetupContext)) {
        ui_core::uikit::run_app_reconnectable(setup);
    }

    #[cfg(not(any(
        target_os = "linux",
        target_os = "ios",
        all(target_os = "macos", feature = "gtk")
    )))]
    pub fn run(_app_id: &str, setup: fn(&mut SetupContext)) {
        use ui_core::widgets::Platform;
        <crate::DefaultPlatform as Platform>::run_app(setup);
    }
}
