mod assets;
mod i18n;
mod qualifier;
#[cfg(feature = "reactive")]
pub mod reactive;

pub use assets::{AssetDescriptor, AssetVariant, BinaryData};
pub use i18n::{Message, TranslationData, replace_param};
pub use qualifier::{ColorScheme, Density, QualifierSet, ResourceContext};
pub use unic_langid::LanguageIdentifier;

/// Generate a type-safe message struct implementing [`Message`].
///
/// Use this when you need a specific parameter type other than `String`
/// (the default emitted by `resources-build`).  The generated struct shadows
/// the build-generated one if placed in the same module.
///
/// ```ignore
/// message!(UnreadMessages, count: i64);
/// ```
#[macro_export]
macro_rules! message {
    ($name:ident) => {
        pub struct $name;
        impl $crate::Message for $name {
            fn apply(&self, template: &str) -> ::std::string::String {
                template.to_owned()
            }
        }
    };

    ($name:ident, $($field:ident : $ty:ty),+ $(,)?) => {
        pub struct $name {
            $(pub $field: $ty),+
        }
        impl $crate::Message for $name {
            fn apply(&self, template: &str) -> ::std::string::String {
                let mut s = template.to_owned();
                $(s = $crate::replace_param(&s, stringify!($field), &self.$field.to_string());)+
                s
            }
        }
    };
}
