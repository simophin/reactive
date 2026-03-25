mod constant;
mod ext;
mod primitives;
pub(crate) mod stored;
pub mod wrapper;

pub use constant::*;
pub use ext::{SignalExt, SignalMapper};
pub(crate) use stored::BoxedStoredSignal;
pub use stored::StoredSignal;

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

impl<T> Signal for Box<dyn Signal<Value = T>> {
    type Value = T;

    fn read(&self) -> Self::Value {
        Box::as_ref(self).read()
    }
}

pub type BoxedSignal<T> = Box<dyn Signal<Value = T>>;
