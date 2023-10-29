use std::{
    any::Any,
    cell::RefCell,
    marker::PhantomData,
    rc::{Rc, Weak},
};

use crate::{
    node::Node,
    registry::{Registry, SignalID},
    tracker::Tracker,
};

pub fn create_signal<T: 'static>(initial_value: impl Into<T>) -> (Signal<T>, SignalWriter<T>) {
    let id = Node::with_current(|n| {
        n.expect("create_signal can be only called within the set up phase")
            .add_signal()
    });

    let value: Box<dyn Any> = Box::new(initial_value.into());
    let state = Rc::new(SignalValue::new(value));

    (
        Signal {
            id,
            value: state.clone(),
            _marker: PhantomData,
        },
        SignalWriter {
            id,
            value: state,
            registry: Rc::downgrade(&Registry::current()),
            _marker: PhantomData,
        },
    )
}

type SignalValue = RefCell<Box<dyn Any>>;
#[derive(Clone)]
pub struct Signal<T> {
    id: SignalID,
    value: Rc<SignalValue>,
    _marker: PhantomData<T>,
}

impl<T: 'static> Signal<T> {
    #[inline]
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.with(T::clone)
    }

    #[inline]
    pub fn with<R>(&self, access: impl FnOnce(&T) -> R) -> R {
        Tracker::track_signal(self.id);
        self.value
            .borrow()
            .downcast_ref()
            .map(access)
            .expect("Value must be of type T")
    }
}

#[derive(Clone)]
pub struct SignalWriter<T> {
    id: SignalID,
    value: Rc<SignalValue>,
    registry: Weak<RefCell<Registry>>,
    _marker: PhantomData<T>,
}

impl<T: 'static> SignalWriter<T> {
    pub fn set(&self, value: impl Into<T>) {
        self.update_with(|v| *v = value.into());
    }

    pub fn update_with(&self, update: impl FnOnce(&mut T)) {
        let Some(registry) = self.registry.upgrade() else {
            return;
        };

        {
            let mut value = self.value.borrow_mut();
            let mut value = value
                .downcast_mut::<T>()
                .expect("Value must be of type T during signal update");

            update(&mut value);
        }

        registry.borrow_mut().notify_signal_changed(self.id);
    }
}
