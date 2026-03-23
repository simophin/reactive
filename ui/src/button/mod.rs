use reactive_core::{Component, Signal};

pub trait ButtonComponent: Component + Sized {
    fn new(title: impl Signal<Value = String> + 'static, on_click: impl Fn() + 'static) -> Self;
}

#[cfg(target_os = "macos")]
mod appkit;
#[cfg(target_os = "macos")]
pub use appkit::*;
