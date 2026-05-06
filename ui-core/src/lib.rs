#[cfg(all(feature = "appkit", not(target_os = "macos")))]
compile_error!("ui-core feature `appkit` is only available on macOS");

#[cfg(all(feature = "gtk", not(any(target_os = "macos", target_os = "linux"))))]
compile_error!("ui-core feature `gtk` is only available on macOS and Linux");

#[cfg(all(feature = "uikit", not(target_os = "ios")))]
compile_error!("ui-core feature `uikit` is only available on iOS");

#[cfg(all(feature = "android", not(target_os = "android")))]
compile_error!("ui-core feature `android` is only available on Android");

/// Utilities for converting between text offset representations across platforms.
///
/// Different platforms use different units for selection offsets:
/// - Apple / Android / Windows: UTF-16 code units
/// - GTK: Unicode codepoints
///
/// Use these when writing cross-platform code that needs to translate between
/// the two representations.
pub mod encoding;

/// Cross-platform layout primitives and logical layout components.
///
/// Logical components (`Padding`, `Center`, `Align`, `SizedBox`, `Expanded`)
/// carry no platform views — they propagate `LayoutHints` via context, which
/// the nearest real view component below them consumes and translates to
/// platform constraints.
pub mod prop;
pub mod widgets;

#[cfg(all(feature = "appkit", target_os = "macos"))]
pub(crate) mod apple;

#[cfg(all(feature = "appkit", target_os = "macos"))]
pub mod appkit;

#[cfg(all(feature = "android", target_os = "android"))]
pub mod android;

#[cfg(all(feature = "gtk", any(target_os = "macos", target_os = "linux")))]
pub mod gtk;
#[cfg(all(feature = "uikit", target_os = "ios"))]
pub mod uikit;

pub use prop::Prop;
