pub mod action_target;
pub mod app_loop;
pub mod bindable;
pub mod prop;
pub mod view_builder;

pub use action_target::ActionTarget;
pub use prop::Prop;
pub use view_builder::ViewBuilder;

// Re-exported so the view_props! macro can reference it via $crate::paste
// without requiring callers to add paste as a direct dependency.
#[doc(hidden)]
pub use paste;

/// Generate `Prop` statics for a view type from a compact property list.
///
/// Derives the setter name from the property name (snake_case → setTitle,
/// setFontSize, etc.) and handles `String → NSString` conversion automatically.
///
/// ```ignore
/// apple::view_props! {
///     Button on NSButton {
///         title: String;
///         enabled: bool;
///         font_size: f64;
///     }
/// }
/// // Generates: PROP_TITLE, PROP_ENABLED, PROP_FONT_SIZE
/// ```
#[macro_export]
macro_rules! view_props {
    // String — needs NSString conversion
    ($component:ident on $view:ident { $name:ident : String ; $($rest:tt)* }) => {
        $crate::paste::paste! {
            pub static [<PROP_ $name:upper>]: &$crate::Prop<$component, $view, String> =
                &$crate::Prop::new(|view, value| {
                    view.[<set $name:camel>](
                        &::objc2_foundation::NSString::from_str(&value)
                    );
                });
        }
        $crate::view_props!($component on $view { $($rest)* });
    };
    // All other types — passed through directly
    ($component:ident on $view:ident { $name:ident : $ty:ty ; $($rest:tt)* }) => {
        $crate::paste::paste! {
            pub static [<PROP_ $name:upper>]: &$crate::Prop<$component, $view, $ty> =
                &$crate::Prop::new(|view, value| {
                    view.[<set $name:camel>](value);
                });
        }
        $crate::view_props!($component on $view { $($rest)* });
    };
    // Base case
    ($component:ident on $view:ident { }) => {};
}
