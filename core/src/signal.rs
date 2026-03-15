use std::marker::PhantomData;

pub(crate) type SignalID = u64;

#[derive(Copy, Clone)]
pub struct Signal<T: 'static> {
    id: SignalID,
    _marker: PhantomData<T>,
}

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
