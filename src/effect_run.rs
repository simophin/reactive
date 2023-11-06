use crate::{
    effect_context::EffectContext,
    react_context::NodeID,
    task::{Task, TaskCleanUp},
    tasks_queue::TaskQueueRef,
    tracker::Tracker,
    util::signal_broadcast::Sender,
};

pub struct EffectRun(Option<TaskCleanUp>);

impl EffectRun {
    pub fn new(
        node_id: NodeID,
        signal_sender: Sender,
        task_queue_handle: &TaskQueueRef,
        mut effect: impl FnMut(&mut EffectContext) + 'static,
    ) -> Self {
        let task = {
            let task_queue_handle = task_queue_handle.clone();
            Task::new_future(async move {
                let mut tracker = Tracker::default();
                let mut signal_receiver = signal_sender.subscribe();
                let mut effect_ctx = EffectContext::new(node_id, signal_sender, task_queue_handle);

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
