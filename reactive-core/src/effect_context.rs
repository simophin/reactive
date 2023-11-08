use futures::Future;

use crate::{
    clean_up::{BoxedCleanUp, CleanUp},
    component::BoxedComponent,
    node::Node,
    react_context::{new_node_id, ReactiveContext},
    setup_context::SetupContext,
    task::Task,
    tasks_queue::TaskQueueRef,
    SetupContextData,
};

pub struct EffectContext {
    setup_data: SetupContextData,
    clean_ups: Vec<BoxedCleanUp>,
}

impl EffectContext {
    pub(crate) fn new(setup_data: SetupContextData) -> Self {
        Self {
            setup_data,
            clean_ups: Default::default(),
        }
    }

    pub fn queue(&self) -> &TaskQueueRef {
        &self.setup_data.queue
    }

    pub fn mount_node(&self, component: BoxedComponent) -> Node {
        SetupContext::new(SetupContextData {
            node_id: new_node_id(),
            ..self.setup_data.clone()
        })
        .mount_node(component)
    }

    pub fn spawn(&mut self, future: impl Future<Output = ()> + 'static) {
        let task = Task::new_future(future);
        let Ok(clean_up) = self.setup_data.queue.queue_task(task) else {
            log::warn!("Effect task queue is dropped before the effect is run");
            return;
        };

        self.add_clean_up(clean_up);
    }

    pub fn with_current_node(&mut self, task: impl FnOnce(&mut Node) + 'static) {
        let node_id = self.setup_data.node_id;
        self.spawn_reactive_task(move |ctx| {
            if let Some(node) = ctx.find_node(node_id) {
                task(node);
            }
        });
    }

    fn spawn_reactive_task(&mut self, task: impl FnOnce(&mut ReactiveContext) + 'static) {
        let task = Task::new_reactive_context(task);
        let Ok(clean_up) = self.setup_data.queue.queue_task(task) else {
            log::warn!("Effect task queue is dropped before the effect is run");
            return;
        };

        self.add_clean_up(clean_up);
    }

    pub fn add_clean_up(&mut self, cleanup: impl CleanUp) {
        self.clean_ups.push(Box::new(cleanup));
    }

    pub(crate) fn clear(&mut self) {
        for clean_up in self.clean_ups.drain(..) {
            clean_up.clean_up();
        }
    }
}

impl Drop for EffectContext {
    fn drop(&mut self) {
        self.clear();
    }
}
