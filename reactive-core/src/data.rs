use std::any::Any;
use std::marker::PhantomData;
use std::sync::atomic::AtomicUsize;

use smallvec::SmallVec;

#[derive(Default)]
pub struct UserDataMap(SmallVec<[(&'static UserDataInner, Box<dyn Any>); 3]>);

impl UserDataMap {
    pub fn get<T>(&self, key: &'static UserDataKey<T>) -> Option<&T> {
        self.0
            .binary_search_by_key(&key.id, |(id, _)| *id)
            .ok()
            .map(|index| {
                let (_, value) = &self.0[index];
                value.downcast_ref().unwrap()
            })
    }

    pub fn put<T>(&mut self, key: &'static UserDataKey<T>, value: T) -> Option<T> {
        match self.0.binary_search_by_key(&key.id, |(id, _)| *id) {
            Ok(index) => {
                let value: Box<dyn Any> = Box::new(value);
                let mut value = (key.id, value);
                std::mem::swap(&mut self.0[index], &mut value);
                Some(*value.1.downcast().unwrap())
            }

            Err(index) => {
                self.0.insert(index, (key.id, Box::new(value)));
                None
            }
        }
    }
}

pub struct UserDataKey<T> {
    id: ID,
    _marker: PhantomData<T>,
}

impl<T> UserDataKey<T> {
    pub fn new() -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

        Self {
            id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            _marker: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static KEY: UserDataKey<i32> = UserDataKey::new();

    #[test]
    fn user_data_map_works() {
        let mut map = UserDataMap::default();

        assert_eq!(map.get(&KEY), None);

        assert_eq!(map.put(&KEY, 1), None);
        assert_eq!(map.get(&KEY), Some(&1));

        assert_eq!(map.put(&KEY, 2), Some(1));
        assert_eq!(map.get(&KEY), Some(&2));
    }
}
