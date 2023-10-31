use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use futures::channel::mpsc;

use crate::component::BoxedComponent;
use crate::node::{MountedNode, Node};
use crate::react_context::{ReactiveContext, SignalID};
use crate::task::WeakTask;

pub struct RenderContext(BoxedComponent);

impl RenderContext {
    pub fn new(component: BoxedComponent) -> Self {
        Self(component)
    }

    pub fn setup(self) -> ReadyContext {
        let (sender, receiver) = mpsc::channel(10);
        ReadyContext {
            node: Node::setup_from(sender, self.0),
            signal_change_rx: receiver,
        }
    }
}

pub struct ReadyContext {
    node: Node,
    signal_change_rx: mpsc::Receiver<SignalID>,
}

impl ReadyContext {
    pub fn mount(self) -> MountedContext<impl Future> {
        let mut context = ReactiveContext::default();
        let node = self.node.mount(&mut context);
        MountedContext {
            node,
            context: Some(context),
        }
    }
}

pub struct MountedContext<F> {
    node: MountedNode,
    context: Option<ReactiveContext<F>>,
}

impl<F> MountedContext<F> {
    pub fn unmount(self) -> RenderContext {
        RenderContext::new(
            self.node
                .unmount(&mut self.context.expect("To have context")),
        )
    }

    fn run_pending_tasks_and_effects(&mut self, waker: &Waker) {
        let mut context = self.context.take().expect("To have context");
        context.set_waker(waker);

        let pending_tasks: Vec<WeakTask> = context.take_pending_tasks().collect();
        let mut pending_effects = context.take_pending_effect_runs();

        // Run effects
        let mut changes = vec![];
        ReactiveContext::set_current(Some(context));
        for effect in pending_effects.drain(..) {
            let mut effect = effect.borrow_mut();
            changes.push((effect.id, effect.run()));
        }

        // Run tasks
        for task in pending_tasks {
            //TODO:
        }

        let mut context = ReactiveContext::set_current(None).expect("To have context");

        for (id, change) in changes {
            context.update_signal_deps(id, change);
        }

        self.context.replace(context);
    }

    pub fn wait(&mut self) -> MountedContextFuture<'_, F> {
        MountedContextFuture(self)
    }
}

pub struct MountedContextFuture<'a, T>(&'a mut MountedContext<T>);

impl<'a, F> Future for MountedContextFuture<'a, F> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.0.run_pending_tasks_and_effects(cx.waker());
        Poll::Pending
    }
}
