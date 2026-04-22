use std::ops::Deref;

#[derive(Default, Clone, Debug, PartialEq, Eq)]
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

    pub fn intersects(&self, other: &Self) -> bool {
        if self.0.is_empty() || other.0.is_empty() {
            return false;
        }

        let (small, large) = if self.0.len() <= other.0.len() {
            (&self.0, &other.0)
        } else {
            (&other.0, &self.0)
        };

        // Binary search wins when k*log(n) < k+n. Use floor log2 via leading_zeros.
        let log2_large = (usize::BITS - large.len().leading_zeros()) as usize;
        if small.len() * log2_large < small.len() + large.len() {
            small.iter().any(|x| large.binary_search(x).is_ok())
        } else {
            let mut i = 0;
            let mut j = 0;
            while i < small.len() && j < large.len() {
                match small[i].cmp(&large[j]) {
                    std::cmp::Ordering::Equal => return true,
                    std::cmp::Ordering::Less => i += 1,
                    std::cmp::Ordering::Greater => j += 1,
                }
            }
            false
        }
    }
}
