use crate::{component::BoxedComponent, tasks_queue::TaskQueueHandle};

pub struct EffectContext {
    pub task_queue_handle: TaskQueueHandle,
}

impl EffectContext {
    pub fn new(task_queue_handle: TaskQueueHandle) -> Self {
        Self { task_queue_handle }
    }

    pub fn queue_children_replacement(&mut self, children: Vec<BoxedComponent>) {
        //todo
    }
}
