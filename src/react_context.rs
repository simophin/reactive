use std::{
    pin::Pin,
    sync::atomic::{AtomicUsize, Ordering},
    task::{Context, Poll},
};

use async_broadcast::{Receiver, Sender};
use futures::Future;

use crate::{
    component::Component,
    effect_run::EffectRun,
    node::Node,
    setup_context::SetupContext,
    tasks_queue::{TaskQueue, TaskQueueHandle},
};

pub type SignalID = usize;
pub type NodeID = usize;

pub struct ReactiveContext {
    tasks: TaskQueue,
    signal_receiver: Receiver<SignalID>,
    signal_sender: Sender<SignalID>,

    root: Option<Node>,
}

impl Default for ReactiveContext {
    fn default() -> Self {
        let (signal_sender, signal_receiver) = async_broadcast::broadcast(256);

        Self {
            tasks: TaskQueue::default(),
            root: Default::default(),
            signal_receiver,
            signal_sender,
        }
    }
}

impl ReactiveContext {
    pub fn poll(&mut self) -> ReactiveContextPoll<'_> {
        ReactiveContextPoll { ctx: self }
    }

    pub fn signal_receiver(&self) -> Receiver<SignalID> {
        self.signal_receiver.clone()
    }

    pub fn task_queue_handle(&self) -> TaskQueueHandle {
        self.tasks.handle()
    }

    pub fn set_root(&mut self, node: Option<Node>) {
        self.root = node;
    }

    pub fn mount_node(&mut self, mut component: impl Component) -> Node {
        let mut ctx = SetupContext::new(self.signal_sender.clone());
        component.setup(&mut ctx);

        let node_id = ctx.node_id();

        // Set up children first
        let children = ctx
            .children
            .into_iter()
            .map(|c| self.mount_node(c))
            .collect();

        // Setup effects
        let effects = ctx
            .effects
            .into_iter()
            .map(|e| EffectRun::new(self.task_queue_handle(), self.signal_receiver(), e))
            .collect();

        Node {
            id: node_id,
            effects,
            clean_ups: ctx.clean_ups,
            children,
        }
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
        tasks.retain_mut(|t| Pin::new(t).poll(cx, self.ctx).is_pending());

        std::mem::swap(&mut self.ctx.tasks.active, &mut tasks);

        Poll::Pending
    }
}

#[derive(Clone)]
pub struct SignalNotifier(SignalID, Sender<SignalID>);

impl SignalNotifier {
    pub fn new(id: SignalID, sender: Sender<SignalID>) -> Self {
        Self(id, sender)
    }

    pub fn signal_id(&self) -> SignalID {
        self.0
    }

    pub fn notify_changed(&mut self) {
        match self.1.try_broadcast(self.0) {
            Ok(_) => {}
            Err(e) => {
                log::warn!("Unable to delivery signal changes due to: {e:?}");
            }
        }
    }
}

pub fn new_signal_id() -> SignalID {
    lazy_static::lazy_static! {
        static ref SIGNAL_ID_SEQ: AtomicUsize = AtomicUsize::new(0);
    }

    SIGNAL_ID_SEQ.fetch_add(1, Ordering::SeqCst)
}

pub fn new_node_id() -> NodeID {
    lazy_static::lazy_static! {
        static ref NODE_ID_SEQ: AtomicUsize = AtomicUsize::new(0);
    }

    NODE_ID_SEQ.fetch_add(1, Ordering::SeqCst)
}
