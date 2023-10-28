use std::any::Any;

pub fn create_signal<T>(initial_value: T) -> (Signal<T>, SignalWriter<T>) {
    todo!()
}

pub struct Signal<T>(T);

pub struct SignalWriter<T>(T);

impl<T: Clone> Signal<T> {
    pub fn get(&self) -> &T {
        todo!()
    }
}

pub struct SignalState {
    current_value: Box<dyn Any>,
}
