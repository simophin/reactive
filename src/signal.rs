use std::{any::Any, marker::PhantomData, sync::Arc};

use parking_lot::RwLock;

use crate::{
    react_context::{SignalID, SignalNotifier},
    tracker::Tracker,
};

pub trait Signal: Clone + 'static {
    type Value: 'static;

    fn get(&self) -> Self::Value
    where
        Self::Value: Clone;

    fn with<R>(&self, access: impl FnOnce(&Self::Value) -> R) -> R;
}

impl<F, T> Signal for F
where
    F: Fn() -> T + Clone + 'static,
    T: 'static,
{
    type Value = T;

    #[inline]
    fn get(&self) -> T
    where
        Self::Value: Clone,
    {
        self()
    }

    #[inline]
    fn with<R>(&self, access: impl FnOnce(&T) -> R) -> R {
        access(&self())
    }
}

type SignalValue = Arc<RwLock<Box<dyn Any>>>;

pub struct SignalReader<T> {
    id: SignalID,
    value: SignalValue,
    _marker: PhantomData<T>,
}

pub fn signal_pair<T: 'static>(
    id: SignalID,
    value: T,
    notifier: SignalNotifier,
) -> (SignalReader<T>, SignalWriter<T>) {
    let value = SignalValue::new(RwLock::new(Box::new(value)));
    (
        SignalReader {
            id,
            value: value.clone(),
            _marker: PhantomData,
        },
        SignalWriter {
            value,
            notifier,
            _marker: PhantomData,
        },
    )
}

impl<T: 'static> Signal for SignalReader<T> {
    type Value = T;

    #[inline]
    fn get(&self) -> T
    where
        Self::Value: Clone,
    {
        self.with(T::clone)
    }

    #[inline]
    fn with<R>(&self, access: impl FnOnce(&T) -> R) -> R {
        Tracker::track_signal(self.id);
        self.value
            .read()
            .downcast_ref()
            .map(access)
            .expect("Value must be of type T")
    }
}

impl<T> Clone for SignalReader<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            value: self.value.clone(),
            _marker: PhantomData,
        }
    }
}

#[derive(Clone)]
pub struct SignalWriter<T> {
    value: SignalValue,
    notifier: SignalNotifier,
    _marker: PhantomData<T>,
}

impl<T: 'static> SignalWriter<T> {
    pub fn set(&mut self, value: T)
    where
        T: Eq,
    {
        self.update_with(|old_value| {
            let changed = old_value != &value;
            *old_value = value;
            changed
        });
    }

    pub fn update_with(&mut self, update: impl FnOnce(&mut T) -> bool) {
        {
            let mut value = self.value.write();
            let mut value = value
                .downcast_mut::<T>()
                .expect("Value must be of type T during signal update");

            update(&mut value);
        }

        self.notifier.notify_changed();
    }
}
