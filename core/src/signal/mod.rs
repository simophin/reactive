mod constant;
mod dynamic;
mod ext;
pub(crate) mod primitives;
pub(crate) mod stored;
mod wrapper;

pub use constant::*;
pub use dynamic::*;
pub use ext::{SignalExt, SignalMapper};
pub use stored::{ReadStoredSignal, StoredSignal};
pub use wrapper::SignalWrapper;

/// A reactive signal. Object-safe: `dyn Signal<Value = T>` is valid when `T: Clone + 'static`.
///
pub trait Signal {
    type Value;

    /// Read the current value, cloning it out.
    fn read(&self) -> Self::Value;
}

/// Unit is a constant signal — useful as a no-op input to streams/resources.
impl Signal for () {
    type Value = ();
    fn read(&self) {}
}

/// Any `Fn() -> T` is a computed signal that re-evaluates on every read.
impl<T, F: Fn() -> T> Signal for F {
    type Value = T;

    fn read(&self) -> T {
        self()
    }
}

impl<S: Signal> Signal for Option<S> {
    type Value = Option<S::Value>;

    fn read(&self) -> Self::Value {
        Some(self.as_ref()?.read())
    }
}
