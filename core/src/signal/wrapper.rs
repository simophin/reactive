use crate::Signal;

pub struct SignalWrapper<S, T>(S, T);

impl<S, T> SignalWrapper<S, T> {
    pub fn new(signal: S, value: T) -> Self {
        Self(signal, value)
    }
}

impl<S, T, V> Signal for SignalWrapper<S, T>
where
    S: Signal,
    T: Fn(S::Value) -> V,
{
    type Value = V;

    fn read(&self) -> Self::Value {
        (self.1)(self.0.read())
    }
}

impl<S> Signal for SignalWrapper<S, ()>
where
    S: Signal,
{
    type Value = S::Value;

    fn read(&self) -> Self::Value {
        self.0.read()
    }
}
