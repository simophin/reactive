use crate::{Signal, SignalExt};
use std::any::Any;
use std::marker::PhantomData;
use std::rc::Rc;

#[derive(Clone)]
pub struct BoxedSignal(Rc<dyn Signal<Value = Box<dyn Any>>>);

impl BoxedSignal {
    pub fn typed<T: Clone + 'static>(self) -> TypedBoxedSignal<T> {
        TypedBoxedSignal(self, PhantomData)
    }
}

impl<S: Signal + 'static> From<S> for BoxedSignal {
    fn from(value: S) -> Self {
        BoxedSignal(Rc::new(value.map_value(|v| Box::new(v) as Box<dyn Any>)))
    }
}

pub struct TypedBoxedSignal<T>(BoxedSignal, PhantomData<T>);

impl<T> Clone for TypedBoxedSignal<T> {
    fn clone(&self) -> Self {
        TypedBoxedSignal(self.0.clone(), PhantomData)
    }
}

impl<T: Clone + 'static> Signal for TypedBoxedSignal<T> {
    type Value = T;

    fn read(&self) -> T {
        let value = self.0.0.read();
        value
            .downcast_ref::<T>()
            .cloned()
            .expect("Boxed signal value was not of the expected type")
    }
}
