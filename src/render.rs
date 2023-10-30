use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;
use std::task::Waker;

use crate::component::BoxedComponent;
use crate::node::{MountedNode, Node};
use crate::react_context::ReactiveContext;

pub struct RenderContext(BoxedComponent);

impl RenderContext {
    pub fn new(component: BoxedComponent) -> Self {
        Self(component)
    }

    pub fn setup(self) -> ReadyContext {
        ReadyContext {
            node: Node::setup_from(self.1),
        }
    }
}

pub struct ReadyContext {
    node: Node,
}

impl ReadyContext {
    pub fn mount(self) -> MountedContext {
        let mut context = ReactiveContext::default();
        let node = self.node.mount(&mut context);
        MountedContext {
            node,
            context: Some(context),
        }
    }
}

pub struct MountedContext {
    node: MountedNode,
    context: Option<ReactiveContext>,
}

impl MountedContext {
    pub fn unmount(self) -> RenderContext {
        RenderContext::new(
            self.node
                .unmount(&mut self.context.expect("To have context")),
        )
    }

    fn run_pending_tasks_and_effects(&mut self, waker: &Waker) {
        let mut context = self.context.take().expect("To have context");
        context.set_waker(waker);

        // Run effects
        ReactiveContext::set_current(Some(context));
        
    }
}

impl Future for MountedContext {
    type Output = ();

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.run_pending_tasks_and_effects(cx.waker());
        std::task::Poll::Pending
    }
}
