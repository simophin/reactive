use reactive_core::Component;

pub trait RowComponent: Component + Sized {
    fn new() -> Self;
    fn child(self, c: impl Component + 'static) -> Self;
}

pub trait ColumnComponent: Component + Sized {
    fn new() -> Self;
    fn child(self, c: impl Component + 'static) -> Self;
}

#[cfg(target_os = "macos")]
mod appkit;

#[cfg(target_os = "macos")]
pub use appkit::*;
