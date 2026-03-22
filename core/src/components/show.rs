use crate::component::{BoxedComponent, Component, SetupContext};
use crate::components::Switch;

pub struct Show {
    switch: Switch,
}

impl Show {
    pub fn new(
        condition: impl FnMut() -> bool + 'static,
        then: impl FnMut() -> BoxedComponent + 'static,
        otherwise: impl FnMut() -> BoxedComponent + 'static,
    ) -> Self {
        Self {
            switch: Switch::new(otherwise).case(condition, then),
        }
    }
}

impl Component for Show {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        Box::new(self.switch).setup(ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ReactiveScope, Signal};
    use futures::task::noop_waker_ref;
    use std::sync::{Arc, Mutex};
    use std::task::Context;

    #[test]
    fn test_show_initially_visible() {
        let mut scope = ReactiveScope::default();
        let root = scope.create_child_component(None);
        let visible = scope.create_signal(true);
        let log = Arc::new(Mutex::new(Vec::<&str>::new()));

        let show = Box::new(Show::new(
            move || visible.read(),
            {
                let log = Arc::clone(&log);
                move || -> BoxedComponent {
                    let log = Arc::clone(&log);
                    Box::new(move |_: &mut SetupContext| log.lock().unwrap().push("shown"))
                }
            },
            || -> BoxedComponent { Box::new(()) },
        ));

        show.setup(&mut SetupContext {
            scope: &mut scope,
            component_id: root,
        });
        assert_eq!(*log.lock().unwrap(), vec!["shown"]);
    }

    #[test]
    fn test_show_initially_hidden() {
        let mut scope = ReactiveScope::default();
        let root = scope.create_child_component(None);
        let visible = scope.create_signal(false);
        let log = Arc::new(Mutex::new(Vec::<&str>::new()));

        let show = Box::new(Show::new(
            move || visible.read(),
            {
                let log = Arc::clone(&log);
                move || -> BoxedComponent {
                    let log = Arc::clone(&log);
                    Box::new(move |_: &mut SetupContext| log.lock().unwrap().push("shown"))
                }
            },
            {
                let log = Arc::clone(&log);
                move || -> BoxedComponent {
                    let log = Arc::clone(&log);
                    Box::new(move |_: &mut SetupContext| log.lock().unwrap().push("hidden"))
                }
            },
        ));

        show.setup(&mut SetupContext {
            scope: &mut scope,
            component_id: root,
        });
        assert_eq!(*log.lock().unwrap(), vec!["hidden"]);
    }

    #[test]
    fn test_show_toggle() {
        let mut scope = ReactiveScope::default();
        let root = scope.create_child_component(None);
        let visible = scope.create_signal(true);
        let log = Arc::new(Mutex::new(Vec::<&str>::new()));

        let show = Box::new(Show::new(
            {
                let visible = visible.clone();
                move || visible.read()
            },
            {
                let log = Arc::clone(&log);
                move || -> BoxedComponent {
                    let log = Arc::clone(&log);
                    Box::new(move |_: &mut SetupContext| log.lock().unwrap().push("shown"))
                }
            },
            {
                let log = Arc::clone(&log);
                move || -> BoxedComponent {
                    let log = Arc::clone(&log);
                    Box::new(move |_: &mut SetupContext| log.lock().unwrap().push("hidden"))
                }
            },
        ));

        show.setup(&mut SetupContext {
            scope: &mut scope,
            component_id: root,
        });
        assert_eq!(*log.lock().unwrap(), vec!["shown"]);

        visible.update_if_changes(false);
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*log.lock().unwrap(), vec!["shown", "hidden"]);

        visible.update_if_changes(true);
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*log.lock().unwrap(), vec!["shown", "hidden", "shown"]);
    }

    #[test]
    fn test_show_same_state_no_rerun() {
        let mut scope = ReactiveScope::default();
        let root = scope.create_child_component(None);
        let visible = scope.create_signal(true);
        let count = scope.create_signal(0);
        let log = Arc::new(Mutex::new(Vec::<&str>::new()));

        let show = Box::new(Show::new(
            {
                let count = count.clone();
                move || {
                    let _ = count.read();
                    visible.read()
                }
            },
            {
                let log = Arc::clone(&log);
                move || -> BoxedComponent {
                    let log = Arc::clone(&log);
                    Box::new(move |_: &mut SetupContext| log.lock().unwrap().push("shown"))
                }
            },
            || -> BoxedComponent { Box::new(()) },
        ));

        show.setup(&mut SetupContext {
            scope: &mut scope,
            component_id: root,
        });
        assert_eq!(*log.lock().unwrap(), vec!["shown"]);

        count.update_if_changes(1);
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*log.lock().unwrap(), vec!["shown"]);
    }
}
