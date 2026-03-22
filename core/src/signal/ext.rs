use crate::BoxedSignal;

pub trait SignalExt: super::Signal {
    /// Create a derived signal by applying `map_fn` to the value.
    /// Requires `Self: Sized` so it stays off the trait-object vtable.
    fn map<T, F>(self, map_fn: F) -> SignalMapper<Self, F>
    where
        Self: Sized,
        T: Clone + 'static,
        F: Fn(Self::Value) -> T + 'static,
    {
        SignalMapper {
            orig_signal: self,
            map_fn,
        }
    }

    fn boxed(self) -> BoxedSignal<Self::Value>
    where
        Self: Sized + 'static,
        Self::Value: Clone + 'static,
    {
        Box::new(self)
    }
}

impl<S: super::Signal> SignalExt for S {}

/// A signal derived from another by applying a mapping function.
/// Created via [`crate::signal::Signal::map`].
#[derive(Clone)]
pub struct SignalMapper<S, F> {
    orig_signal: S,
    map_fn: F,
}

impl<S, T, F> super::Signal for SignalMapper<S, F>
where
    S: super::Signal,
    F: Fn(S::Value) -> T,
{
    type Value = T;

    fn read(&self) -> T {
        (self.map_fn)(self.orig_signal.read())
    }
}
