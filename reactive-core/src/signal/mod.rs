mod rw;

pub trait Signal: 'static {
    type Value;

    fn with<R>(&self, access: impl for<'a> FnOnce(&Self::Value) -> R) -> R;
}

impl<F, T> Signal for F
where
    F: Fn() -> T + 'static,
    T: 'static,
{
    type Value = T;

    #[inline]
    fn with<R>(&self, access: impl FnOnce(&T) -> R) -> R {
        access(&self())
    }
}

impl Signal for () {
    type Value = ();

    #[inline]
    fn with<R>(&self, access: impl FnOnce(&Self::Value) -> R) -> R {
        access(&())
    }
}

pub trait SignalGetter: Signal {
    fn get(&self) -> Self::Value;
}

impl<S> SignalGetter for S
where
    S: Signal,
    <S as Signal>::Value: Clone,
{
    fn get(&self) -> <S as Signal>::Value {
        self.with(<S as Signal>::Value::clone)
    }
}

pub use rw::*;
