use crate::render::Scope;

pub fn on_clean_up(func: impl FnOnce() + 'static) {
    Scope::with_current(move |s| {
        s.expect("on_clean_up can be only called within the set up phase")
            .add_clean_up_func(func);
    });
}
