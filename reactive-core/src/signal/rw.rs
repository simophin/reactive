use std::{any::Any, cell::RefCell, marker::PhantomData, rc::Rc};

use crate::{
    react_context::{SignalID, SignalNotifier},
    tracker::Tracker,
};

use super::Signal;

type ValueRef = Rc<RefCell<Box<dyn Any>>>;

pub struct SignalReader<T>(ValueRef, SignalID, PhantomData<T>);

impl<T> Clone for SignalReader<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone(), PhantomData)
    }
}

impl<T: 'static> Signal for SignalReader<T> {
    type Value = T;

    #[inline]
    fn with<R>(&self, access: impl FnOnce(&Self::Value) -> R) -> R {
        let value = self.0.borrow();
        let value = value.downcast_ref::<T>().expect("To have correct type");
        let r = access(value);
        Tracker::track_signal(self.1);
        r
    }

    fn id(&self) -> Option<SignalID> {
        Some(self.1)
    }
}

#[derive(Clone)]
pub struct SignalWriter<T> {
    value: ValueRef,
    notifier: SignalNotifier,
    _marker: PhantomData<T>,
}

impl<T: 'static> SignalWriter<T> {
    pub fn update_with(&mut self, update: impl FnOnce(&mut T) -> bool) {
        let changed = update(
            self.value
                .borrow_mut()
                .downcast_mut::<T>()
                .expect("To have correct type"),
        );

        if changed {
            self.notifier.notify_changed();
        }
    }

    pub fn update_if_modified(&mut self, value: T)
    where
        T: Eq,
    {
        self.update_with(|old_value| {
            let changed = old_value != &value;
            *old_value = value;
            changed
        });
    }

    pub fn update(&mut self, value: T) {
        self.update_with(|old_value| {
            *old_value = value;
            true
        });
    }
}

pub fn signal<T: 'static>(
    initial_value: T,
    notifier: SignalNotifier,
) -> (SignalReader<T>, SignalWriter<T>) {
    let value: Box<dyn Any> = Box::new(initial_value);
    let value = Rc::new(RefCell::new(value));
    (
        SignalReader(value.clone(), notifier.signal_id(), PhantomData),
        SignalWriter {
            value,
            notifier,
            _marker: PhantomData,
        },
    )
}
