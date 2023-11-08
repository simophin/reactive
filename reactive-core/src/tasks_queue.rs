use std::{
    cell::RefCell,
    collections::BTreeSet,
    rc::{Rc, Weak},
    task::Context,
};

use local_waker::LocalWaker;

use crate::{
    clean_up::CleanUp,
    task::{Task, TaskID},
};

#[derive(Default)]
struct PendingQueue {
    adding: Vec<Task>,
    removing: BTreeSet<TaskID>,
    waker: LocalWaker,
}

#[derive(Default)]
pub struct TaskQueue {
    pending: Rc<RefCell<PendingQueue>>,
    pub active: Vec<Option<Task>>,
}

impl TaskQueue {
    pub fn apply_pending(&mut self, cx: &Context<'_>) {
        // Removing a task needs to happen outside of the pending mutable borrow scope
        let mut removing;

        {
            let mut pending = self.pending.borrow_mut();
            self.active
                .extend(pending.adding.drain(..).map(|t| Some(t)));

            // Reuse memory inside pending.adding for recording the tasks being removed
            removing = std::mem::take(&mut pending.adding);

            // Go through active tasks and remove the ones we need
            for task in &mut self.active {
                if let Some(t) = task {
                    if pending.removing.contains(&t.id()) {
                        removing.push(task.take().unwrap());
                    }
                }
            }
            pending.removing.clear();
            pending.waker.register(cx.waker());
        }
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

    fn queue_task_removal(&self, id: TaskID) {
        if let Some(inner) = self.0.upgrade() {
            let mut inner = inner.borrow_mut();
            inner.removing.insert(id);
            inner.waker.wake();
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

impl CleanUp for TaskCleanUp {
    fn clean_up(mut self: Box<Self>) {
        self.do_clean_up();
    }
}

pub fn new_task_id() -> TaskID {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}
