use crate::tasks_queue::TaskQueueRef;

pub struct EffectContext {
    pub task_queue_handle: TaskQueueRef,
}

impl EffectContext {
    pub fn new(task_queue_handle: TaskQueueRef) -> Self {
        Self { task_queue_handle }
    }
}
