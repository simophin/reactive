use std::{
    cell::RefCell,
    collections::BTreeSet,
    rc::{Rc, Weak},
    task::Context,
};

use local_waker::LocalWaker;

use crate::task::{Task, TaskCleanUp, TaskID};

#[derive(Default)]
struct PendingQueue {
    adding: Vec<Task>,
    removing: BTreeSet<TaskID>,
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
        self.active.append(&mut pending.adding);
        self.active
            .retain(|task| !pending.removing.contains(&task.id()));
        pending.adding.clear();
        pending.removing.clear();
        pending.waker.register(cx.waker());
    }

    pub fn handle(&self) -> TaskQueueRef {
        TaskQueueRef(Rc::downgrade(&self.pending))
    }
}

#[derive(Clone)]
pub struct TaskQueueRef(Weak<RefCell<PendingQueue>>);

impl TaskQueueRef {
    pub fn queue_task(&self, task: Task) -> Result<TaskCleanUp, Task> {
        let Some(inner) = self.0.upgrade() else {
            return Err(task);
        };

        let clean_up = TaskCleanUp::new(self.clone(), task.id());

        let mut inner = inner.borrow_mut();
        inner.adding.push(task);
        inner.waker.wake();
        Ok(clean_up)
    }

    pub fn queue_task_removal(&self, id: TaskID) {
        if let Some(inner) = self.0.upgrade() {
            let mut inner = inner.borrow_mut();
            inner.removing.insert(id);
            inner.waker.wake();
        }
    }
}
