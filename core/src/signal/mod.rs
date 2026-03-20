pub mod ext;
mod stored;

pub(crate) use stored::BoxedStoredSignal;
pub use stored::StoredSignal;

pub type SignalId = u64;

/// A reactive signal. Object-safe: `dyn Signal<Value = T>` is valid when `T: Clone + 'static`.
///
/// The `access`-based zero-cost pattern has been dropped in favour of always cloning, which
/// allows the trait to be used as a trait object — essential when binding large numbers of
/// UI properties dynamically.
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

pub type BoxedSignal<T> = Box<dyn Signal<Value = T>>;
