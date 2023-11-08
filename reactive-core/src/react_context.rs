use std::{
    pin::Pin,
    sync::atomic::{AtomicUsize, Ordering},
    task::{Context, Poll},
};

use futures::Future;

use crate::{
    component::BoxedComponent,
    node::Node,
    setup_context::SetupContext,
    tasks_queue::{TaskQueue, TaskQueueRef},
    util::{signal_broadcast::Sender, vec::VecExt},
    SetupContextData,
};

pub(crate) type SignalID = usize;
pub type NodeID = usize;

#[derive(Default)]
pub struct ReactiveContext {
    tasks: TaskQueue,
    signal_sender: Sender,

    root: Option<Node>,
}

impl ReactiveContext {
    pub fn poll(&mut self) -> ReactiveContextPoll<'_> {
        ReactiveContextPoll { ctx: self }
    }

    pub fn task_queue_handle(&self) -> TaskQueueRef {
        self.tasks.handle()
    }

    pub fn set_root(&mut self, node: Option<Node>) {
        self.root = node;
    }

    pub fn mount_node(&mut self, component: BoxedComponent) -> Node {
        SetupContext::new(SetupContextData {
            node_id: new_node_id(),
            queue: self.task_queue_handle(),
            signal_sender: self.signal_sender.clone(),
            context_map: Default::default(),
        })
        .mount_node(component)
    }

    pub fn find_node(&mut self, id: NodeID) -> Option<&mut Node> {
        self.root.as_mut().and_then(|root| root.find_by(id))
    }
}

pub struct ReactiveContextPoll<'a> {
    ctx: &'a mut ReactiveContext,
}

impl<'a> Future for ReactiveContextPoll<'a> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.ctx.tasks.apply_pending(cx);

        let mut tasks = vec![];
        std::mem::swap(&mut self.ctx.tasks.active, &mut tasks);

        // Poll all the task and remove the completed ones
        for task in &mut tasks {
            if let Some(t) = task.as_mut() {
                if Pin::new(t).poll(cx, self.ctx).is_ready() {
                    *task = None;
                }
            }
        }

        tasks.condense();
        std::mem::swap(&mut self.ctx.tasks.active, &mut tasks);

        Poll::Pending
    }
}

#[derive(Clone)]
pub struct SignalNotifier(SignalID, Sender);

impl SignalNotifier {
    pub fn new(id: SignalID, sender: Sender) -> Self {
        Self(id, sender)
    }

    pub fn signal_id(&self) -> SignalID {
        self.0
    }

    pub fn notify_changed(&mut self) {
        self.1.send(self.0);
    }
}

pub fn new_signal_id() -> SignalID {
    static SIGNAL_ID_SEQ: AtomicUsize = AtomicUsize::new(0);
    SIGNAL_ID_SEQ.fetch_add(1, Ordering::SeqCst)
}

pub fn new_node_id() -> NodeID {
    static NODE_ID_SEQ: AtomicUsize = AtomicUsize::new(0);
    NODE_ID_SEQ.fetch_add(1, Ordering::SeqCst)
}
