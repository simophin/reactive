use std::{any::Any, cell::RefCell, marker::PhantomData, rc::Rc};

use futures::{select, Future, FutureExt};

use crate::{
    signal::{Signal, SignalGetter},
    task::{Task, TaskCleanUp},
    tasks_queue::TaskQueueRef,
    tracker::Tracker,
    util::{mpsc, signal_broadcast::Receiver},
};

pub struct ResourceRun {
    clean_up: TaskCleanUp,
    value: Rc<RefCell<Option<Box<dyn Any>>>>,
}

impl ResourceRun {
    pub fn as_signal<T: 'static>(&self) -> ResourceAccess<T> {
        ResourceAccess(self.value.clone(), PhantomData)
    }

    pub fn new<S, I, F, T: 'static>(
        queue: &TaskQueueRef,
        mut signal_receiver: Receiver,
        signal: S,
        factory: F,
    ) -> (Self, mpsc::Sender<()>)
    where
        S: for<'a> Signal<Value<'a> = &'a I>,
        I: Clone + 'static,
        F: ResourceFactory<I, T> + 'static,
    {
        let value: Rc<RefCell<Option<Box<dyn Any>>>> = Default::default();

        let (trigger_tx, mut trigger_rx) = mpsc::channel::<()>(10);

        let task = Task::new_future({
            let value = value.clone();
            async move {
                let mut tracker = Tracker::default();
                loop {
                    Tracker::set_current(Some(tracker));
                    let input = signal.get();
                    tracker = Tracker::set_current(None).expect("To have tracker set before");
                    signal_receiver.set_subscribing(tracker.iter());

                    select! {
                        out = factory.create(input).fuse() => {
                            value.borrow_mut().replace(Box::new(out));
                        }

                        r = signal_receiver.next().fuse() => {
                            if r.is_none() {
                                break;
                            }
                        }

                        v = trigger_rx.recv().fuse() => {
                            if v.is_none() {
                                break;
                            }
                        }
                    }
                }
            }
        });

        let task_id = task.id();
        let _ = queue.queue_task(task);
        (
            Self {
                clean_up: TaskCleanUp::new(queue.clone(), task_id),
                value,
            },
            trigger_tx,
        )
    }
}

pub trait ResourceFactory<Input: 'static, Output: 'static> {
    type Fut: Future<Output = Output> + 'static;

    fn create(&self, input: Input) -> Self::Fut;
}

pub struct ResourceAccess<T>(Rc<RefCell<Option<Box<dyn Any>>>>, PhantomData<T>);

impl<T> Clone for ResourceAccess<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl<T: 'static> Signal for ResourceAccess<T> {
    type Value<'a> = Option<&'a T>;

    fn with<R>(&self, access: impl FnOnce(Option<&T>) -> R) -> R {
        self.0
            .borrow()
            .as_ref()
            .map(|v| v.downcast_ref())
            .map(access)
            .unwrap()
    }
}
