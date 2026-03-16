use std::marker::PhantomData;

pub(crate) type SignalID = u64;

pub struct Signal<T: 'static> {
    id: SignalID,
    _marker: PhantomData<T>,
}

impl<T: 'static> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            _marker: PhantomData,
        }
    }
}

impl<T: 'static> Copy for Signal<T> {}

impl<T: 'static> Signal<T> {
    pub(crate) fn new(id: SignalID) -> Self {
        Self {
            id,
            _marker: PhantomData,
        }
    }

    pub fn id(&self) -> SignalID {
        self.id
    }
}
