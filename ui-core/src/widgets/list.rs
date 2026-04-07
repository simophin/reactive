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

pub trait ListComparator<T> {
    fn are_content_the_same(&self, a: &T, b: &T) -> bool;
    fn is_same_item(&self, a: &T, b: &T) -> bool;
}

struct EqualComparator;

impl<T: PartialEq> ListComparator<T> for EqualComparator {
    fn are_content_the_same(&self, a: &T, b: &T) -> bool {
        a == b
    }

    fn is_same_item(&self, a: &T, b: &T) -> bool {
        a == b
    }
}

pub trait List: Component + Sized {
    fn new<L, I, C>(
        list_data: impl Signal<Value = L> + 'static,
        component_factory: impl FnMut(ReadStoredSignal<I>) -> C + 'static,
    ) -> Self
    where
        L: ListData<I> + 'static,
        C: Component + 'static,
        I: Clone + PartialEq + 'static,
    {
        Self::new_with_comparator(list_data, component_factory, EqualComparator)
    }

    fn new_with_comparator<L, I, C, Comp>(
        list_data: impl Signal<Value = L> + 'static,
        component_factory: impl FnMut(ReadStoredSignal<I>) -> C + 'static,
        list_comparator: Comp,
    ) -> Self
    where
        L: ListData<I> + 'static,
        C: Component + 'static,
        I: Clone + 'static,
        Comp: ListComparator<I> + 'static;

    fn orientation(self, orientation: ListOrientation) -> Self;
}
