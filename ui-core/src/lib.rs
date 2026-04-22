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

pub use prop::Prop;
pub use view_builder::ViewBuilder;
