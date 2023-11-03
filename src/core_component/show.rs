use crate::{
    component::{boxed_component, BoxedComponent, Component, ComponentFactory},
    effect::Effect,
    effect_context::EffectContext,
    react_context::NodeID,
    setup_context::SetupContext,
    task::{Task, TaskCleanUp},
};

pub struct Show<F, CS, CF>(Option<ShowData<F, CS, CF>>);

struct ShowData<F, CS, CF> {
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
    pub fn new(test: F, success: CS, fail: CF) -> BoxedComponent {
        boxed_component(Self(Some(ShowData {
            test,
            success,
            fail,
        })))
    }
}

struct ShowEffect<F, CS, CF> {
    node_id: NodeID,
    data: ShowData<F, CS, CF>,
    last_success: Option<bool>,
}

impl<F, CS, CF> Effect for ShowEffect<F, CS, CF>
where
    F: FnMut() -> bool + 'static,
    CS: ComponentFactory,
    CF: ComponentFactory,
{
    type Cleanup = Option<TaskCleanUp>;

    fn run(&mut self, ctx: &mut EffectContext) -> Self::Cleanup {
        let new_success = (self.data.test)();
        log::debug!(
            "ShowEffect: new_success={new_success}, last={:?}",
            self.last_success
        );
        match (self.last_success, new_success) {
            (Some(last), new) if last == new => return None,
            _ => {}
        }

        self.last_success.replace(new_success);

        let child: BoxedComponent = if new_success {
            Box::new(self.data.success.create())
        } else {
            Box::new(self.data.fail.create())
        };

        let node_id = self.node_id;

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
    }
}

impl<F, CS, CF> Component for Show<F, CS, CF>
where
    F: FnMut() -> bool + 'static,
    CS: ComponentFactory,
    CF: ComponentFactory,
{
    fn setup(&mut self, ctx: &mut SetupContext) {
        let data = self
            .0
            .take()
            .expect("Setup called twice for Show component");

        let node_id = ctx.node_id();

        ctx.create_effect(ShowEffect {
            node_id,
            data,
            last_success: None,
        });
    }
}
