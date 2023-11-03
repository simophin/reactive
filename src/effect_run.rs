use crate::{
    effect::{Effect, EffectCleanup},
    effect_context::EffectContext,
    task::{Task, TaskCleanUp},
    tasks_queue::TaskQueueRef,
    tracker::Tracker,
    util::signal_broadcast::Receiver,
};

pub struct EffectRun {
    _clean_up: TaskCleanUp,
}

impl EffectRun {
    pub fn new(
        task_queue_handle: &TaskQueueRef,
        mut signal_receiver: Receiver,
        mut effect: impl Effect,
    ) -> Self {
        let task = {
            let mut task_queue_handle = task_queue_handle.clone();
            Task::new_future(async move {
                let mut tracker = Tracker::default();
                let mut _last_clean_up: AutoEffectCleanUp<_>;

                loop {
                    tracker.clear();
                    Tracker::set_current(Some(tracker));
                    let mut effect_ctx = EffectContext::new(task_queue_handle);
                    _last_clean_up = AutoEffectCleanUp::new(effect.run(&mut effect_ctx));
                    task_queue_handle = effect_ctx.task_queue_handle;
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

        let _clean_up = TaskCleanUp::new(task_queue_handle.clone(), task.id());
        let _ = task_queue_handle.queue_task(task);

        Self { _clean_up }
    }
}

impl Drop for EffectRun {
    fn drop(&mut self) {
        log::debug!("EffectRun dropped");
    }
}

struct AutoEffectCleanUp<C: EffectCleanup>(Option<C>);

impl<C> AutoEffectCleanUp<C>
where
    C: EffectCleanup,
{
    fn new(clean_up: C) -> Self {
        Self(Some(clean_up))
    }
}

impl<C> Drop for AutoEffectCleanUp<C>
where
    C: EffectCleanup,
{
    fn drop(&mut self) {
        self.0.take().unwrap().cleanup();
    }
}
