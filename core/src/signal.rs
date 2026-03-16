use slotmap::new_key_type;
use std::marker::PhantomData;

new_key_type! {
    pub struct SignalId;
}

pub struct Signal<T: 'static> {
    id: SignalId,
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
    pub(crate) fn new(id: SignalId) -> Self {
        Self {
            id,
            _marker: PhantomData,
        }
    }

    pub fn id(&self) -> SignalId {
        self.id
    }
}
