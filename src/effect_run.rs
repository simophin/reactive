use crate::{
    effect_context::EffectContext,
    task::{Task, TaskCleanUp},
    tasks_queue::TaskQueueRef,
    tracker::Tracker,
    util::signal_broadcast::Receiver,
};

pub struct EffectRun(Option<TaskCleanUp>);

impl EffectRun {
    pub fn new(
        task_queue_handle: &TaskQueueRef,
        mut signal_receiver: Receiver,
        mut effect: impl FnMut(&mut EffectContext) + 'static,
    ) -> Self {
        let task = {
            let task_queue_handle = task_queue_handle.clone();
            Task::new_future(async move {
                let mut tracker = Tracker::default();
                let mut effect_ctx = EffectContext::new(task_queue_handle);

                loop {
                    effect_ctx.clear();

                    tracker.clear();
                    Tracker::set_current(Some(tracker));
                    effect(&mut effect_ctx);
                    tracker = Tracker::set_current(None).expect("To have tracker back");

                    signal_receiver.set_subscribing(tracker.iter());

                    // Wait for signal changes
                    loop {
                        if signal_receiver.next().await.is_some() {
                            break;
                        } else {
                            return;
                        }
                    }
                }
            })
        };

        let Ok(clean_up) = task_queue_handle.queue_task(task) else {
            log::warn!("Effect task queue is dropped before the effect is run");
            return Self(None);
        };

        Self(Some(clean_up))
    }
}
