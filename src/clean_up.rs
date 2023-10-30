use crate::node_ref::NodeRef;

pub fn on_clean_up(clean_up: impl CleanUp) {
    NodeRef::with_current(move |s| {
        s.expect("on_clean_up can be only called within the set up phase")
            .add_clean_up_func(clean_up);
    });
}

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
