use std::{any::Any, cmp, marker::PhantomData, rc::Rc};

use smallvec::SmallVec;

use crate::Signal;

#[derive(Default, Clone)]
pub struct ContextMap(SmallVec<[(&'static ContextKeyInner, ContextSignal); 3]>);

impl ContextMap {
    pub fn insert<T: 'static>(
        &mut self,
        key: &'static ContextKey<T>,
        value: impl Signal<Value = T>,
    ) {
        let value = ContextSignal(Rc::new(move |access| {
            value.with(move |value| (*access)(value as &dyn Any))
        }));

        match self.0.binary_search_by_key(&&key.0, |entry| entry.0) {
            Ok(index) => self.0[index] = (&key.0, value),
            Err(index) => self.0.insert(index, (&key.0, value)),
        }
    }

    pub fn get<T: 'static>(&self, key: &'static ContextKey<T>) -> Option<impl Signal<Value = T>> {
        self.0
            .binary_search_by_key(&&key.0, |entry| entry.0)
            .ok()
            .and_then(|index| self.0.get(index))
            .map(|(_, signal)| signal.clone().to_signal())
    }
}

struct ContextSignal(Rc<dyn Fn(&mut dyn for<'a> FnMut(&'a dyn Any))>);

impl Clone for ContextSignal {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

struct ContextAsSignal<T>(ContextSignal, PhantomData<T>);

impl<T> Clone for ContextAsSignal<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl ContextSignal {
    fn to_signal<T: 'static>(self) -> ContextAsSignal<T> {
        ContextAsSignal(self, PhantomData)
    }
}

impl<T: 'static> Signal for ContextAsSignal<T> {
    type Value = T;

    fn with<R>(&self, access: impl for<'a> FnOnce(&Self::Value) -> R) -> R {
        let mut result = None;
        let mut access = Some(access);
        (self.0 .0)(&mut |value: &dyn Any| {
            result.replace((access.take().unwrap())(value.downcast_ref::<T>().unwrap()));
        });

        result.unwrap()
    }
}

type ContextKeyInnerRef = &'static ContextKeyInner;

#[derive(Clone, Copy)]
struct ContextKeyInner;

impl PartialEq<ContextKeyInnerRef> for ContextKeyInnerRef {
    fn eq(&self, other: &ContextKeyInnerRef) -> bool {
        *self as *const ContextKeyInner == *other as *const ContextKeyInner
    }
}

impl Eq for ContextKeyInnerRef {}

impl PartialOrd<ContextKeyInnerRef> for ContextKeyInnerRef {
    fn partial_cmp(&self, other: &ContextKeyInnerRef) -> Option<cmp::Ordering> {
        (*self as *const ContextKeyInner).partial_cmp(&(*other as *const ContextKeyInner))
    }
}

impl Ord for ContextKeyInnerRef {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        (*self as *const ContextKeyInner).cmp(&(*other as *const ContextKeyInner))
    }
}

pub struct ContextKey<T>(ContextKeyInner, PhantomData<T>);

impl<T: Clone + 'static> ContextKey<T> {
    pub const fn new() -> Self {
        Self(ContextKeyInner, PhantomData)
    }
}
