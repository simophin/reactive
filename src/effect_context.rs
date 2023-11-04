use futures::Future;

use crate::{
    clean_up::{BoxedCleanUp, CleanUp},
    react_context::ReactiveContext,
    task::Task,
    tasks_queue::TaskQueueRef,
};

pub struct EffectContext {
    task_queue_handle: TaskQueueRef,
    clean_ups: Vec<BoxedCleanUp>,
}

impl EffectContext {
    pub fn new(task_queue_handle: TaskQueueRef) -> Self {
        Self {
            task_queue_handle,
            clean_ups: Default::default(),
        }
    }

    pub fn queue(&self) -> &TaskQueueRef {
        &self.task_queue_handle
    }

    pub fn spawn(&mut self, future: impl Future<Output = ()> + 'static) {
        let task = Task::new_future(future);
        let Ok(clean_up) = self.task_queue_handle.queue_task(task) else {
            log::warn!("Effect task queue is dropped before the effect is run");
            return;
        };

        self.add_clean_up(clean_up);
    }

    pub fn spawn_reactive_task(&mut self, task: impl FnOnce(&mut ReactiveContext) + 'static) {
        let task = Task::new_reactive_context(task);
        let Ok(clean_up) = self.task_queue_handle.queue_task(task) else {
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
