use std::{any::Any, cell::RefCell, marker::PhantomData, rc::Rc};

use crate::{
    node_ref::{NodeRef, WeakNodeRef},
    registry::SignalID,
    tracker::Tracker,
};

pub fn create_signal<T: 'static>(initial_value: T) -> (SignalReader<T>, SignalWriter<T>) {
    let id = NodeRef::with_current(|n| {
        n.expect("create_signal can be only called within the set up phase")
            .add_signal()
    });

    let value: Box<dyn Any> = Box::new(initial_value);
    let state = Rc::new(SignalValue::new(value));

    (
        SignalReader {
            id,
            value: state.clone(),
            _marker: PhantomData,
        },
        SignalWriter {
            id,
            value: state,
            node: NodeRef::require_current(),
            _marker: PhantomData,
        },
    )
}

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

type SignalValue = RefCell<Box<dyn Any>>;

pub struct SignalReader<T> {
    id: SignalID,
    value: Rc<SignalValue>,
    _marker: PhantomData<T>,
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
            .borrow()
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
    id: SignalID,
    value: Rc<SignalValue>,
    node: WeakNodeRef,
    _marker: PhantomData<T>,
}

impl<T: 'static> SignalWriter<T> {
    pub fn set(&self, value: impl Into<T>) {
        self.update_with(|v| *v = value.into());
    }

    pub fn update_with(&self, update: impl FnOnce(&mut T)) {
        let Some(node) = self.node.upgrade() else {
            return;
        };

        {
            let mut value = self.value.borrow_mut();
            let mut value = value
                .downcast_mut::<T>()
                .expect("Value must be of type T during signal update");

            update(&mut value);
        }

        node.notify_signal_changed(self.id);
    }
}
