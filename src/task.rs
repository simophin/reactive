use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::Future;

use crate::{effect::EffectCleanup, react_context::ReactiveContext, tasks_queue::TaskQueueRef};

pub type TaskID = usize;

pub struct Task {
    id: TaskID,
    content: TaskContent,
}

enum TaskContent {
    Future(Pin<Box<dyn Future<Output = ()>>>),
    ReactiveContext(Option<Box<dyn FnOnce(&mut ReactiveContext) + 'static>>),
}

impl Task {
    pub fn new_future(future: impl Future<Output = ()> + 'static) -> Self {
        Self {
            id: new_task_id(),
            content: TaskContent::Future(Box::pin(future)),
        }
    }

    pub fn new_reactive_context(
        task: impl for<'a> FnOnce(&'a mut ReactiveContext) + 'static,
    ) -> Self {
        Self {
            id: new_task_id(),
            content: TaskContent::ReactiveContext(Some(Box::new(task))),
        }
    }

    pub fn id(&self) -> TaskID {
        self.id
    }

    pub fn poll(
        &mut self,
        cx: &mut Context<'_>,
        reactive_context: &mut ReactiveContext,
    ) -> Poll<()> {
        match &mut self.content {
            TaskContent::Future(future) => future.as_mut().poll(cx),
            TaskContent::ReactiveContext(task) => {
                if let Some(task) = task.take() {
                    task(reactive_context);
                }
                Poll::Ready(())
            }
        }
    }
}

pub struct TaskCleanUp(Option<(TaskQueueRef, TaskID)>);

impl TaskCleanUp {
    pub fn new(queue: TaskQueueRef, id: TaskID) -> Self {
        Self(Some((queue, id)))
    }

    fn do_clean_up(&mut self) {
        if let Some((queue, id)) = self.0.take() {
            queue.queue_task_removal(id);
        }
    }
}

impl Drop for TaskCleanUp {
    fn drop(&mut self) {
        self.do_clean_up();
    }
}

impl EffectCleanup for TaskCleanUp {
    fn cleanup(mut self) {
        self.do_clean_up();
    }
}

pub fn new_task_id() -> TaskID {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}
