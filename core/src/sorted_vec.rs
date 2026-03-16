use std::ops::Deref;

#[derive(Default, Clone, Debug)]
pub struct SortedVec<T>(Vec<T>);

impl<T> Deref for SortedVec<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Ord> From<Vec<T>> for SortedVec<T> {
    fn from(mut vec: Vec<T>) -> Self {
        vec.sort();
        Self(vec)
    }
}

impl<T: Ord> SortedVec<T> {
    pub fn insert(&mut self, mut item: T) -> Option<T> {
        match self.0.binary_search(&item) {
            Ok(index) => {
                std::mem::swap(&mut self.0[index], &mut item);
                Some(item)
            }

            Err(index) => {
                self.0.insert(index, item);
                None
            }
        }
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn contains(&self, item: &T) -> bool {
        self.0.binary_search(item).is_ok()
    }

    pub fn intersects(&self, other: &Self) -> bool {
        let mut i = 0;
        let mut j = 0;

        while i < self.0.len() && j < other.0.len() {
            if self.0[i] == other.0[j] {
                return true;
            } else if self.0[i] < other.0[j] {
                i += 1;
            } else {
                j += 1;
            }
        }

        false
    }
}
