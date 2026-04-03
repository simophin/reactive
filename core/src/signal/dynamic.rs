use crate::Signal;
use std::any::Any;
use std::rc::Rc;

#[derive(Clone)]
pub struct TypeErasedSignal(Rc<dyn Any>, &'static str);

impl TypeErasedSignal {
    pub fn typed<T: Clone + 'static>(&self) -> Rc<dyn Signal<Value = T>> {
        if let Some(value) = self.0.downcast_ref::<Rc<dyn Signal<Value = T>>>() {
            value.clone()
        } else {
            panic!(
                "Unable to extract erased signal to type: {}, actual value is: {:?}",
                std::any::type_name::<T>(),
                self.1
            )
        }
    }
}

impl<S: Signal + 'static> From<S> for TypeErasedSignal {
    fn from(value: S) -> Self {
        let signal: Rc<dyn Signal<Value = S::Value>> = Rc::new(value);
        TypeErasedSignal(Rc::new(signal), std::any::type_name::<S::Value>())
    }
}
