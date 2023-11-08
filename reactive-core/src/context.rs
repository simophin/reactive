use std::{
    any::Any,
    marker::PhantomData,
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
};

use smallvec::SmallVec;

use crate::SignalReader;

pub type ContextID = usize;

#[derive(Default, Clone)]
pub struct ContextMap(SmallVec<[(ContextID, Rc<dyn Any>); 3]>);

impl ContextMap {
    pub fn insert<T: 'static>(&mut self, key: ContextKey<T>, value: SignalReader<T>) {
        match self.0.binary_search_by_key(&key.0, |entry| entry.0) {
            Ok(index) => self.0[index] = (key.0, Rc::new(value)),
            Err(index) => self.0.insert(index, (key.0, Rc::new(value))),
        }
    }

    pub fn get<T: 'static>(&self, key: ContextKey<T>) -> Option<&SignalReader<T>> {
        self.0
            .binary_search_by_key(&key.0, |entry| entry.0)
            .ok()
            .and_then(|index| self.0.get(index))
            .and_then(|(_, context)| context.downcast_ref())
    }
}

#[derive(Clone, Copy)]
pub struct ContextKey<T>(ContextID, PhantomData<T>);

impl<T: 'static> ContextKey<T> {
    pub fn new() -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        Self(id, PhantomData)
    }
}
