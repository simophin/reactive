use super::Signal;

#[derive(Clone)]
pub struct ConstantSignal<T>(pub T);

impl<T> Signal for ConstantSignal<T>
where
    T: Clone + 'static,
{
    type Value = T;

    fn read(&self) -> Self::Value {
        self.0.clone()
    }
}

pub trait IntoSignal {
    type S: Signal;

    fn into_signal(self) -> Self::S;
}

impl<T> IntoSignal for T
where
    T: Clone + 'static,
{
    type S = ConstantSignal<T>;

    fn into_signal(self) -> Self::S {
        ConstantSignal(self)
    }
}
