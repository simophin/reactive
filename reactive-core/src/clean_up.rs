use std::any::Any;

pub trait CleanUp: 'static {
    fn clean_up(self: Box<Self>);
}

pub type BoxedCleanUp = Box<dyn CleanUp>;

impl<F> CleanUp for F
where
    F: FnOnce() + 'static,
{
    fn clean_up(self: Box<Self>) {
        self()
    }
}

impl CleanUp for Box<dyn Any> {
    fn clean_up(self: Box<Self>) {}
}
