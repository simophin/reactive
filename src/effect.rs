use crate::node::Node;

pub fn create_effect(func: impl FnMut() + 'static) {
    Node::with_current(|s| {
        s.expect("create_effect can be only called within the set up phase")
            .add_effect(func);
    });
}
