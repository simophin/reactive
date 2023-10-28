use crate::render::Scope;

pub fn create_effect(func: impl Fn() + 'static) {
    Scope::with_current(|s| {
        s.expect("create_effect can be only called within the set up phase")
            .add_effect(func);
    });
}
