use crate::node::Node;

pub fn on_clean_up(clean_up: impl CleanUp) {
    Node::with_current(move |s| {
        s.expect("on_clean_up can be only called within the set up phase")
            .borrow_mut()
            .add_clean_up_func(clean_up);
    });
}

pub trait CleanUp: 'static {
    fn clean_up(&mut self);
}

impl<F> CleanUp for F
where
    F: FnOnce() + 'static,
{
    fn clean_up(&mut self) {
        self()
    }
}
