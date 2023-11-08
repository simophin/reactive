use crate::{
    effect::Effect,
    effect_context::EffectContext,
    task::{Task, TaskCleanUp},
    tracker::Tracker,
    SetupContextData,
};

pub struct EffectRun(Option<TaskCleanUp>);

impl EffectRun {
    pub(crate) fn new(data: SetupContextData, mut effect: impl Effect) -> Self {
        let task_queue_handle = data.queue.clone();

        let task = {
            Task::new_future(async move {
                let mut tracker = Tracker::default();
                let mut signal_receiver = data.signal_sender.subscribe();
                let mut effect_ctx = EffectContext::new(data.clone());

                loop {
                    effect_ctx.clear();

                    tracker.clear();
                    tracker = tracker.with_current(|| effect.run(&mut effect_ctx)).0;

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
