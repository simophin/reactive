use std::{
    cell::RefCell,
    rc::{Rc, Weak},
    task::Context,
};

use local_waker::LocalWaker;

use crate::task::Task;

#[derive(Default)]
struct PendingQueue {
    tasks: Vec<Task>,
    waker: LocalWaker,
}

#[derive(Default)]
pub struct TaskQueue {
    pending: Rc<RefCell<PendingQueue>>,
    pub active: Vec<Task>,
}

impl TaskQueue {
    pub fn apply_pending(&mut self, cx: &Context<'_>) {
        let mut pending = self.pending.borrow_mut();
        self.active.append(&mut pending.tasks);
        pending.tasks.clear();
        pending.waker.register(cx.waker());
    }

    pub fn handle(&self) -> TaskQueueHandle {
        TaskQueueHandle(Rc::downgrade(&self.pending))
    }
}

#[derive(Clone)]
pub struct TaskQueueHandle(Weak<RefCell<PendingQueue>>);

impl TaskQueueHandle {
    pub fn queue_task(&self, task: Task) -> Result<(), Task> {
        let Some(inner) = self.0.upgrade() else {
            return Err(task);
        };

        let mut inner = inner.borrow_mut();
        inner.tasks.push(task);
        inner.waker.wake();
        Ok(())
    }
}
