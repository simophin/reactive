use reactive_core::{Component, ReadStoredSignal, Signal};

pub trait ListData<T> {
    fn count(&self) -> usize;
    fn get_item(&self, index: usize) -> Option<&T>;
}

impl<T, A> ListData<T> for A
where
    A: AsRef<[T]>,
{
    fn count(&self) -> usize {
        self.as_ref().len()
    }

    fn get_item(&self, index: usize) -> Option<&T> {
        self.as_ref().get(index)
    }
}

pub enum ListOrientation {
    Vertical,
    Horizontal,
}

pub trait List: Component + Sized {
    fn new<L, I, C>(
        list_data: impl Signal<Value = L> + 'static,
        component_factory: impl FnMut(ReadStoredSignal<I>) -> C + 'static,
    ) -> Self
    where
        L: ListData<I> + 'static,
        C: Component + 'static,
        I: 'static;

    fn orientation(self, orientation: ListOrientation) -> Self;
}
