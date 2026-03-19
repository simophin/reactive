use crate::Signal;

#[derive(Clone)]
pub struct SignalMapper<S, F> {
    pub(crate) orig_signal: S,
    pub(crate) map_fn: F,
}

impl<S, T, F> Signal for SignalMapper<S, F>
where
    T: 'static,
    S: Signal,
    F: Fn(&S::Value) -> T + 'static,
{
    type Value = T;

    fn access<R>(&self, f: impl for<'a> FnOnce(&'a Self::Value) -> R) -> R {
        self.orig_signal.access(|v| f(&(self.map_fn)(v)))
    }
}
