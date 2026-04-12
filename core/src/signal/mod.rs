mod constant;
mod ext;
pub(crate) mod primitives;
pub(crate) mod stored;
mod wrapper;

pub use constant::*;
pub use ext::{SignalExt, SignalMapper};
use std::any::Any;
use std::rc::Rc;
pub use stored::{ReadStoredSignal, StoredSignal};
pub use wrapper::SignalWrapper;

/// A reactive signal. Object-safe: `dyn Signal<Value = T>` is valid when `T: Clone + 'static`.
///
pub trait Signal: Any {
    type Value: 'static;

    /// Read the current value, cloning it out.
    fn read(&self) -> Self::Value;

    fn as_any(&self) -> &dyn Any
    where
        Self: Sized + 'static,
    {
        self
    }
}

/// Unit is a constant signal — useful as a no-op input to streams/resources.
impl Signal for () {
    type Value = ();
    fn read(&self) {}
}

impl<T: 'static> Signal for Box<dyn Signal<Value = T>> {
    type Value = T;

    fn read(&self) -> Self::Value {
        self.as_ref().read()
    }
}

impl<T: 'static> Signal for Rc<dyn Signal<Value = T>> {
    type Value = T;

    fn read(&self) -> Self::Value {
        self.as_ref().read()
    }
}

/// Any `Fn() -> T` is a computed signal that re-evaluates on every read.
impl<T: 'static, F: Fn() -> T + 'static> Signal for F {
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
