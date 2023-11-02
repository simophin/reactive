use std::sync::atomic::{AtomicBool, Ordering};

use crate::{
    component::{boxed_component, BoxedComponent, Component, ComponentFactory},
    effect_context::EffectContext,
    setup_context::SetupContext,
    task::Task,
};

pub struct Show<F, CS, CF>(Option<ShowData<F, CS, CF>>);

struct ShowData<F, CS, CF> {
    test: F,
    success: CS,
    fail: CF,
}

impl<F, CS, CF> Show<F, CS, CF>
where
    F: FnMut() -> bool + 'static,
    CS: ComponentFactory,
    CF: ComponentFactory,
{
    pub fn new(test: F, success: CS, fail: CF) -> BoxedComponent {
        boxed_component(Self(Some(ShowData {
            test,
            success,
            fail,
        })))
    }
}

impl<F, CS, CF> Component for Show<F, CS, CF>
where
    F: FnMut() -> bool + 'static,
    CS: ComponentFactory,
    CF: ComponentFactory,
{
    fn setup(&mut self, ctx: &mut SetupContext) {
        let mut data = self
            .0
            .take()
            .expect("Setup called twice for Show component");

        let last_success = AtomicBool::new((data.test)());

        ctx.children.push(if last_success.load(Ordering::Relaxed) {
            Box::new(data.success.create())
        } else {
            Box::new(data.fail.create())
        });

        let node_id = ctx.node_id();

        ctx.create_effect(move |ctx: &mut EffectContext| {
            let new_success = (data.test)();
            if new_success == last_success.load(Ordering::Relaxed) {
                return None;
            }

            let child: BoxedComponent = if new_success {
                Box::new(data.success.create())
            } else {
                Box::new(data.fail.create())
            };

            let (task, handle) = Task::new_reactive_context(move |r| {
                let child = r.mount_node(child);
                if let Some(node) = r.find_node(node_id) {
                    node.children.clear();
                    node.children.push(child);
                }
            });

            let _ = ctx.task_queue_handle.queue_task(task);

            Some(handle)
        });
    }
}
