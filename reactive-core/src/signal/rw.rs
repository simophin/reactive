use std::{cell::RefCell, rc::Rc};

use crate::{
    react_context::{SignalID, SignalNotifier},
    tracker::Tracker,
};

use super::Signal;

type ValueRef<T> = Rc<RefCell<T>>;

pub struct SignalReader<T> {
    value: ValueRef<T>,
    id: SignalID,
}

impl<T> Clone for SignalReader<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            id: self.id,
        }
    }
}

impl<T: 'static> Signal for SignalReader<T> {
    type Value = T;

    #[inline]
    fn with<R>(&self, access: impl FnOnce(&Self::Value) -> R) -> R {
        let r = access(&*self.value.borrow());
        Tracker::track_signal(self.id);
        r
    }
}

pub struct SignalWriter<T> {
    value: ValueRef<T>,
    notifier: SignalNotifier,
}

impl<T> Clone for SignalWriter<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            notifier: self.notifier.clone(),
        }
    }
}

impl<T: 'static> SignalWriter<T> {
    pub fn update_with(&mut self, update: impl FnOnce(&mut T) -> bool) {
        let changed = update(&mut *self.value.borrow_mut());

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
    let value = Rc::new(RefCell::new(initial_value));
    (
        SignalReader {
            value: value.clone(),
            id: notifier.signal_id(),
        },
        SignalWriter { value, notifier },
    )
}
