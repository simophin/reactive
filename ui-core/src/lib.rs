extern crate self as apple;
extern crate self as ui_core;

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
pub mod view_builder;
pub mod widgets;

#[cfg(any(feature = "appkit", feature = "uikit"))]
pub mod action_target;
#[cfg(any(feature = "appkit", feature = "uikit"))]
pub mod app_loop;
#[cfg(any(feature = "appkit", feature = "uikit"))]
pub mod apple_text_input_state;

#[cfg(all(feature = "appkit", target_os = "macos"))]
pub mod appkit;
#[cfg(all(feature = "gtk", any(target_os = "macos", target_os = "linux")))]
pub mod gtk;
#[cfg(all(feature = "uikit", target_os = "ios"))]
pub mod uikit;
#[cfg(all(feature = "android", target_os = "android"))]
pub mod android;

pub use prop::Prop;
pub use view_builder::ViewBuilder;

#[cfg(any(feature = "appkit", feature = "uikit"))]
pub use action_target::ActionTarget;
#[cfg(any(feature = "appkit", feature = "uikit"))]
pub use apple_text_input_state::TextInputState;

#[doc(hidden)]
pub use paste;

/// Generate Apple platform `Prop` statics from a compact property list.
#[macro_export]
macro_rules! view_props {
    ($component:ident on $view:ident { $vis:vis $name:ident : String ; $($rest:tt)* }) => {
        $crate::paste::paste! {
            $vis static [<PROP_ $name:upper>]: $crate::Prop<$component, $view, String> =
                $crate::Prop::new(|view, value| {
                    view.[<set $name:camel>](
                        &::objc2_foundation::NSString::from_str(&value)
                    );
                });
        }
        $crate::view_props!($component on $view { $($rest)* });
    };
    ($component:ident on $view:ident { $vis:vis $name:ident : $ty:ty ; $($rest:tt)* }) => {
        $crate::paste::paste! {
            $vis static [<PROP_ $name:upper>]: $crate::Prop<$component, $view, $ty> =
                $crate::Prop::new(|view, value| {
                    view.[<set $name:camel>](value);
                });
        }
        $crate::view_props!($component on $view { $($rest)* });
    };
    ($component:ident on $view:ident { }) => {};
}
