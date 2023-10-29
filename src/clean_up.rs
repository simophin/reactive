use crate::node::Node;

pub fn on_clean_up(func: impl FnOnce() + 'static) {
    Node::with_current(move |s| {
        s.expect("on_clean_up can be only called within the set up phase")
            .add_clean_up_func(func);
    });
}
