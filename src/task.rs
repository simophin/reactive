use std::{
    cell::RefCell,
    pin::Pin,
    rc::{Rc, Weak},
    task::{Context, Poll},
};

use futures::Future;

use crate::react_context::ReactiveContext;

pub type FutureTask = RefCell<Pin<Box<dyn Future<Output = ()>>>>;
pub type ReactiveContextTask = RefCell<Option<Box<dyn FnOnce(&mut ReactiveContext) + 'static>>>;

pub struct Task(pub Weak<TaskHandleInner>);

pub enum TaskHandleInner {
    Future(FutureTask),
    ReactiveContext(ReactiveContextTask),
}

pub struct TaskHandle(Rc<TaskHandleInner>);

impl Task {
    pub fn new_future(future: impl Future<Output = ()> + 'static) -> (Self, TaskHandle) {
        let future: Box<dyn Future<Output = ()>> = Box::new(future);
        let handle = TaskHandle(Rc::new(TaskHandleInner::Future(RefCell::new(
            future.into(),
        ))));

        (Self(Rc::downgrade(&handle.0)), handle)
    }

    pub fn new_reactive_context(
        task: impl for<'a> FnOnce(&'a mut ReactiveContext) + 'static,
    ) -> (Self, TaskHandle) {
        let handle = TaskHandle(Rc::new(TaskHandleInner::ReactiveContext(RefCell::new(
            Some(Box::new(task)),
        ))));

        (Self(Rc::downgrade(&handle.0)), handle)
    }

    pub fn poll(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        reactive_context: &mut ReactiveContext,
    ) -> Poll<()> {
        let Some(inner) = self.0.upgrade() else {
            return Poll::Ready(());
        };

        match &*inner {
            TaskHandleInner::Future(future) => {
                let mut future = future.borrow_mut();
                future.as_mut().poll(cx)
            }

            TaskHandleInner::ReactiveContext(task) => {
                if let Some(mut task) = task.borrow_mut().take() {
                    task(reactive_context);
                }
                Poll::Ready(())
            }
        }
    }
}
