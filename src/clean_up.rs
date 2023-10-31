pub trait CleanUp: 'static {
    fn clean_up(&mut self);
}

pub type BoxedCleanUp = Box<dyn CleanUp>;

impl CleanUp for BoxedCleanUp {
    fn clean_up(&mut self) {
        self.as_mut().clean_up()
    }
}

impl<F> CleanUp for F
where
    F: FnMut() + 'static,
{
    fn clean_up(&mut self) {
        self()
    }
}
