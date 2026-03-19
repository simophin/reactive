use crate::component::{BoxedComponent, Component, SetupContext};

type ConditionFn = Box<dyn FnMut() -> bool>;
type ComponentFactory = Box<dyn FnMut() -> BoxedComponent>;

struct Case {
    condition: ConditionFn,
    component: ComponentFactory,
}

pub struct Switch {
    cases: Vec<Case>,
    fallback: ComponentFactory,
}

impl Switch {
    pub fn new(fallback: impl FnMut() -> BoxedComponent + 'static) -> Self {
        Self {
            cases: Vec::new(),
            fallback: Box::new(fallback),
        }
    }

    pub fn case(
        mut self,
        condition: impl FnMut() -> bool + 'static,
        component: impl FnMut() -> BoxedComponent + 'static,
    ) -> Self {
        self.cases.push(Case {
            condition: Box::new(condition),
            component: Box::new(component),
        });
        self
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum ActiveBranch {
    Case(usize),
    Fallback,
}

impl Component for Switch {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let mut cases = self.cases;
        let mut fallback = self.fallback;
        let my_id = ctx.component_id();

        ctx.create_effect(move |scope, prev_branch: Option<ActiveBranch>| {
            // Evaluate conditions to find the matching branch
            let new_branch = cases
                .iter_mut()
                .position(|case| (case.condition)())
                .map(ActiveBranch::Case)
                .unwrap_or(ActiveBranch::Fallback);

            let new_component_factory = match (prev_branch, new_branch) {
                (Some(old), new) if old == new => None,
                (_, ActiveBranch::Fallback) => Some(&mut fallback),
                (_, ActiveBranch::Case(idx)) => Some(&mut cases[idx].component),
            };

            let Some(component) = new_component_factory.map(|c| c()) else {
                return new_branch;
            };

            scope.dispose_all_children(my_id);
            component.setup(&mut SetupContext {
                component_id: scope.create_child_component(Some(my_id)),
                scope,
            });

            new_branch
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ReactiveScope;
    use futures::task::noop_waker_ref;
    use std::sync::{Arc, Mutex};
    use std::task::Context;

    #[test]
    fn test_switch_initial_match() {
        let mut scope = ReactiveScope::default();
        let root = scope.create_child_component(None);
        let mode = scope.create_signal("a");

        let log = Arc::new(Mutex::new(Vec::<&str>::new()));

        let switch = Box::new(
            Switch::new()
                .case(move |ctx| ctx.read(mode) == "a", {
                    let log = Arc::clone(&log);
                    move || -> BoxedComponent {
                        let log = Arc::clone(&log);
                        Box::new(move |_: &mut SetupContext| log.lock().unwrap().push("branch_a"))
                    }
                })
                .case(move |ctx| ctx.read(mode) == "b", {
                    let log = Arc::clone(&log);
                    move || -> BoxedComponent {
                        let log = Arc::clone(&log);
                        Box::new(move |_: &mut SetupContext| log.lock().unwrap().push("branch_b"))
                    }
                }),
        );

        switch.setup(&mut SetupContext {
            scope: &mut scope,
            component_id: root,
        });
        assert_eq!(*log.lock().unwrap(), vec!["branch_a"]);
    }

    #[test]
    fn test_switch_changes_branch() {
        let mut scope = ReactiveScope::default();
        let root = scope.create_child_component(None);
        let mode = scope.create_signal("a");

        let log = Arc::new(Mutex::new(Vec::<&str>::new()));

        let switch = Box::new(
            Switch::new()
                .case(move |ctx| ctx.read(mode) == "a", {
                    let log = Arc::clone(&log);
                    move || -> BoxedComponent {
                        let log = Arc::clone(&log);
                        Box::new(move |_: &mut SetupContext| log.lock().unwrap().push("branch_a"))
                    }
                })
                .case(move |ctx| ctx.read(mode) == "b", {
                    let log = Arc::clone(&log);
                    move || -> BoxedComponent {
                        let log = Arc::clone(&log);
                        Box::new(move |_: &mut SetupContext| log.lock().unwrap().push("branch_b"))
                    }
                }),
        );

        switch.setup(&mut SetupContext {
            scope: &mut scope,
            component_id: root,
        });
        assert_eq!(*log.lock().unwrap(), vec!["branch_a"]);

        scope.update_if_changed(mode, "b");
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*log.lock().unwrap(), vec!["branch_a", "branch_b"]);
    }

    #[test]
    fn test_switch_fallback() {
        let mut scope = ReactiveScope::default();
        let root = scope.create_child_component(None);
        let mode = scope.create_signal("unknown");

        let log = Arc::new(Mutex::new(Vec::<&str>::new()));

        let switch = Box::new(
            Switch::new()
                .case(move |ctx| ctx.read(mode) == "a", {
                    let log = Arc::clone(&log);
                    move || -> BoxedComponent {
                        let log = Arc::clone(&log);
                        Box::new(move |_: &mut SetupContext| log.lock().unwrap().push("branch_a"))
                    }
                })
                .fallback({
                    let log = Arc::clone(&log);
                    move || -> BoxedComponent {
                        let log = Arc::clone(&log);
                        Box::new(move |_: &mut SetupContext| log.lock().unwrap().push("fallback"))
                    }
                }),
        );

        switch.setup(&mut SetupContext {
            scope: &mut scope,
            component_id: root,
        });
        assert_eq!(*log.lock().unwrap(), vec!["fallback"]);
    }

    #[test]
    fn test_switch_no_match_no_fallback() {
        let mut scope = ReactiveScope::default();
        let root = scope.create_child_component(None);
        let mode = scope.create_signal("unknown");

        let log = Arc::new(Mutex::new(Vec::<&str>::new()));

        let switch = Box::new(Switch::new().case(move |ctx| ctx.read(mode) == "a", {
            let log = Arc::clone(&log);
            move || -> BoxedComponent {
                let log = Arc::clone(&log);
                Box::new(move |_: &mut SetupContext| log.lock().unwrap().push("branch_a"))
            }
        }));

        switch.setup(&mut SetupContext {
            scope: &mut scope,
            component_id: root,
        });
        assert!(log.lock().unwrap().is_empty());
    }

    #[test]
    fn test_switch_same_branch_no_rerun() {
        let mut scope = ReactiveScope::default();
        let root = scope.create_child_component(None);
        let mode = scope.create_signal("a");
        let count = scope.create_signal(0);

        let log = Arc::new(Mutex::new(Vec::<&str>::new()));

        let switch = Box::new(Switch::new().case(
            move |ctx| {
                let _ = ctx.read(count);
                ctx.read(mode) == "a"
            },
            {
                let log = Arc::clone(&log);
                move || -> BoxedComponent {
                    let log = Arc::clone(&log);
                    Box::new(move |_: &mut SetupContext| log.lock().unwrap().push("branch_a"))
                }
            },
        ));

        switch.setup(&mut SetupContext {
            scope: &mut scope,
            component_id: root,
        });
        assert_eq!(*log.lock().unwrap(), vec!["branch_a"]);

        scope.update_if_changed(count, 1);
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*log.lock().unwrap(), vec!["branch_a"]);
    }
}
