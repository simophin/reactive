mod rw;

pub trait Signal: 'static {
    type Value;

    fn id(&self) -> Option<SignalID>;
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

    fn id(&self) -> Option<SignalID> {
        None
    }
}

pub trait SignalGetter<T>: Signal {
    fn get(&self) -> T;
}

impl<S> SignalGetter<<S as Signal>::Value> for S
where
    S: Signal,
    <S as Signal>::Value: Clone,
{
    fn get(&self) -> <S as Signal>::Value {
        self.with(<S as Signal>::Value::clone)
    }
}

pub use rw::*;

use crate::react_context::SignalID;
