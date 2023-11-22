use std::any::Any;
use std::marker::PhantomData;

use smallvec::SmallVec;

#[derive(Default)]
pub struct UserDataMap(SmallVec<[(*const KeyInner, Box<dyn Any>); 3]>);

impl UserDataMap {
    pub fn get<T>(&self, key: &'static UserDataKey<T>) -> Option<&T> {
        let inner = &key.inner as *const KeyInner;
        self.0
            .binary_search_by_key(&inner, |(id, _)| *id)
            .ok()
            .map(|index| {
                let (_, value) = &self.0[index];
                value.downcast_ref().unwrap()
            })
    }

    pub fn put<T>(&mut self, key: &'static UserDataKey<T>, value: T) -> Option<T> {
        let inner = &key.inner as *const KeyInner;
        match self.0.binary_search_by_key(&inner, |(id, _)| *id) {
            Ok(index) => {
                let mut value: Box<dyn Any> = Box::new(value);
                std::mem::swap(&mut self.0[index].1, &mut value);
                Some(*value.downcast().unwrap())
            }

            Err(index) => {
                self.0.insert(index, (&key.inner, Box::new(value)));
                None
            }
        }
    }
}

struct KeyInner;

pub struct UserDataKey<T> {
    inner: KeyInner,
    _marker: PhantomData<T>,
}

impl<T> UserDataKey<T> {
    pub const fn new() -> Self {
        Self {
            inner: KeyInner,
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
