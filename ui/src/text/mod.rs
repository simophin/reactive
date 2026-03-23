use reactive_core::{Component, Signal};

pub trait TextComponent: Component + Sized {
    fn new(text: impl Signal<Value = String> + 'static) -> Self;
}

#[cfg(target_os = "macos")]
mod appkit;

#[cfg(target_os = "macos")]
pub use appkit::*;
