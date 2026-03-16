use crate::component::{BoxedComponent, Component, SetupContext};
use crate::component_scope::ComponentId;
use crate::EffectContext;

type ConditionFn = Box<dyn Fn(&mut EffectContext) -> bool>;
type ComponentFactory = Box<dyn Fn() -> BoxedComponent>;

struct Case {
    condition: ConditionFn,
    component: ComponentFactory,
}

pub struct Switch {
    cases: Vec<Case>,
    fallback: Option<ComponentFactory>,
}

impl Switch {
    pub fn new() -> Self {
        Self {
            cases: Vec::new(),
            fallback: None,
        }
    }

    pub fn case(
        mut self,
        condition: impl Fn(&mut EffectContext) -> bool + 'static,
        component: impl Fn() -> BoxedComponent + 'static,
    ) -> Self {
        self.cases.push(Case {
            condition: Box::new(condition),
            component: Box::new(component),
        });
        self
    }

    pub fn fallback(mut self, component: impl Fn() -> BoxedComponent + 'static) -> Self {
        self.fallback = Some(Box::new(component));
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
        let cases = self.cases;
        let fallback = self.fallback;
        let component_id = ctx.component_id();

        ctx.create_effect(move |ectx, active: Option<&mut (Option<ActiveBranch>, Option<ComponentId>)>| {
            // Evaluate conditions to find the matching branch
            let new_branch = cases
                .iter()
                .position(|case| (case.condition)(ectx))
                .map(ActiveBranch::Case)
                .or_else(|| fallback.as_ref().map(|_| ActiveBranch::Fallback));

            let prev_branch = active.as_ref().map(|a| a.0).flatten();
            let prev_child = active.as_ref().map(|a| a.1).flatten();

            // If the active branch hasn't changed, do nothing
            if prev_branch == new_branch {
                return (prev_branch, prev_child);
            }

            // Dispose the previous child component
            if let Some(child_id) = prev_child {
                ectx.dispose_component(child_id);
            }

            // Set up the new branch
            let new_child = new_branch.map(|branch| {
                let mut child_ctx = ectx.setup_child(component_id);
                let child_id = child_ctx.component_id();
                let component = match branch {
                    ActiveBranch::Case(idx) => (cases[idx].component)(),
                    ActiveBranch::Fallback => (fallback.as_ref().unwrap())(),
                };
                component.setup(&mut child_ctx);
                child_id
            });

            (new_branch, new_child)
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

    struct FnComponent<F: FnOnce(&mut SetupContext)>(Option<F>);

    impl<F: FnOnce(&mut SetupContext) + 'static> Component for FnComponent<F> {
        fn setup(mut self: Box<Self>, ctx: &mut SetupContext) {
            if let Some(f) = self.0.take() {
                f(ctx);
            }
        }
    }

    fn fn_component(f: impl FnOnce(&mut SetupContext) + 'static) -> BoxedComponent {
        Box::new(FnComponent(Some(f)))
    }

    #[test]
    fn test_switch_initial_match() {
        let mut scope = ReactiveScope::default();
        let root = scope.create_component(None);
        let mode = scope.create_signal("a");

        let log = Arc::new(Mutex::new(Vec::<&str>::new()));

        let switch = Box::new(Switch::new()
            .case(
                move |ctx| ctx.read(mode) == "a",
                {
                    let log = Arc::clone(&log);
                    move || fn_component({
                        let log = Arc::clone(&log);
                        move |_| log.lock().unwrap().push("branch_a")
                    })
                },
            )
            .case(
                move |ctx| ctx.read(mode) == "b",
                {
                    let log = Arc::clone(&log);
                    move || fn_component({
                        let log = Arc::clone(&log);
                        move |_| log.lock().unwrap().push("branch_b")
                    })
                },
            ));

        switch.setup(&mut SetupContext { scope: &mut scope, component_id: root });
        assert_eq!(*log.lock().unwrap(), vec!["branch_a"]);
    }

    #[test]
    fn test_switch_changes_branch() {
        let mut scope = ReactiveScope::default();
        let root = scope.create_component(None);
        let mode = scope.create_signal("a");

        let log = Arc::new(Mutex::new(Vec::<&str>::new()));

        let switch = Box::new(Switch::new()
            .case(
                move |ctx| ctx.read(mode) == "a",
                {
                    let log = Arc::clone(&log);
                    move || fn_component({
                        let log = Arc::clone(&log);
                        move |_| log.lock().unwrap().push("branch_a")
                    })
                },
            )
            .case(
                move |ctx| ctx.read(mode) == "b",
                {
                    let log = Arc::clone(&log);
                    move || fn_component({
                        let log = Arc::clone(&log);
                        move |_| log.lock().unwrap().push("branch_b")
                    })
                },
            ));

        switch.setup(&mut SetupContext { scope: &mut scope, component_id: root });
        assert_eq!(*log.lock().unwrap(), vec!["branch_a"]);

        scope.update_if_changed(mode, "b");
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*log.lock().unwrap(), vec!["branch_a", "branch_b"]);
    }

    #[test]
    fn test_switch_fallback() {
        let mut scope = ReactiveScope::default();
        let root = scope.create_component(None);
        let mode = scope.create_signal("unknown");

        let log = Arc::new(Mutex::new(Vec::<&str>::new()));

        let switch = Box::new(Switch::new()
            .case(
                move |ctx| ctx.read(mode) == "a",
                {
                    let log = Arc::clone(&log);
                    move || fn_component({
                        let log = Arc::clone(&log);
                        move |_| log.lock().unwrap().push("branch_a")
                    })
                },
            )
            .fallback({
                let log = Arc::clone(&log);
                move || fn_component({
                    let log = Arc::clone(&log);
                    move |_| log.lock().unwrap().push("fallback")
                })
            }));

        switch.setup(&mut SetupContext { scope: &mut scope, component_id: root });
        assert_eq!(*log.lock().unwrap(), vec!["fallback"]);
    }

    #[test]
    fn test_switch_no_match_no_fallback() {
        let mut scope = ReactiveScope::default();
        let root = scope.create_component(None);
        let mode = scope.create_signal("unknown");

        let log = Arc::new(Mutex::new(Vec::<&str>::new()));

        let switch = Box::new(Switch::new()
            .case(
                move |ctx| ctx.read(mode) == "a",
                {
                    let log = Arc::clone(&log);
                    move || fn_component({
                        let log = Arc::clone(&log);
                        move |_| log.lock().unwrap().push("branch_a")
                    })
                },
            ));

        switch.setup(&mut SetupContext { scope: &mut scope, component_id: root });
        assert!(log.lock().unwrap().is_empty());
    }

    #[test]
    fn test_switch_same_branch_no_rerun() {
        let mut scope = ReactiveScope::default();
        let root = scope.create_component(None);
        let mode = scope.create_signal("a");
        let count = scope.create_signal(0);

        let log = Arc::new(Mutex::new(Vec::<&str>::new()));

        let switch = Box::new(Switch::new()
            .case(
                move |ctx| {
                    let _ = ctx.read(count);
                    ctx.read(mode) == "a"
                },
                {
                    let log = Arc::clone(&log);
                    move || fn_component({
                        let log = Arc::clone(&log);
                        move |_| log.lock().unwrap().push("branch_a")
                    })
                },
            ));

        switch.setup(&mut SetupContext { scope: &mut scope, component_id: root });
        assert_eq!(*log.lock().unwrap(), vec!["branch_a"]);

        scope.update_if_changed(count, 1);
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*log.lock().unwrap(), vec!["branch_a"]);
    }
}
