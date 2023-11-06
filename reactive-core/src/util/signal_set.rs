use smallvec::SmallVec;

use crate::react_context::SignalID;

#[derive(Default, Clone, Debug)]
pub struct SignalSet(SmallVec<[SignalID; 3]>);

impl SignalSet {
    pub fn insert(&mut self, signal_id: SignalID) {
        match self.0.binary_search(&signal_id) {
            Ok(_) => {}
            Err(index) => self.0.insert(index, signal_id),
        }
    }

    pub fn contains(&self, signal_id: SignalID) -> bool {
        self.0.binary_search(&signal_id).is_ok()
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = SignalID> + '_ {
        self.0.iter().cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_set_works() {
        let mut set = SignalSet::default();

        set.insert(3);
        set.insert(2);
        set.insert(1);

        // Ensure order is correct
        assert_eq!(set.iter().collect::<Vec<_>>(), vec![1, 2, 3]);

        assert!(!set.contains(4));

        set.clear();

        assert!(!set.contains(1));
        assert!(!set.contains(2));
        assert!(!set.contains(3));
    }
}
