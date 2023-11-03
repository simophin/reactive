use derive_builder::Builder;

use crate::{
    component::{BoxedComponent, Component, ComponentFactory},
    effect::effect_fn,
    setup_context::SetupContext,
    task::{Task, TaskCleanUp},
};

#[derive(Builder)]
pub struct Show<F, CS, CF> {
    test: F,
    success: CS,
    fail: CF,
}

impl<F, CS, CF> Show<F, CS, CF>
where
    F: FnMut() -> bool + 'static,
    CS: ComponentFactory,
    CF: ComponentFactory,
{
    pub fn new(test: F, success: CS, fail: CF) -> impl Component {
        Show {
            test,
            success,
            fail,
        }
    }
}

impl<F, CS, CF> Component for Show<F, CS, CF>
where
    F: FnMut() -> bool + 'static,
    CS: ComponentFactory,
    CF: ComponentFactory,
{
    fn setup(mut self: Box<Self>, ctx: &mut SetupContext) {
        let node_id = ctx.node_id();
        let mut last_success = None;

        ctx.create_effect(effect_fn(move |ctx| {
            let new_success = (self.test)();
            log::debug!("ShowEffect: new_success={new_success}, last={last_success:?}");
            match (last_success, new_success) {
                (Some(last), new) if last == new => return None,
                _ => {}
            }

            last_success.replace(new_success);

            let child: BoxedComponent = if new_success {
                Box::new(self.success.create())
            } else {
                Box::new(self.fail.create())
            };

            let task = Task::new_reactive_context(move |r| {
                let child = r.mount_node(child);
                if let Some(node) = r.find_node(node_id) {
                    node.children.clear();
                    node.children.push(child);
                }
            });

            let cleanup = TaskCleanUp::new(ctx.task_queue_handle.clone(), task.id());
            let _ = ctx.task_queue_handle.queue_task(task);

            Some(cleanup)
        }));
    }
}
