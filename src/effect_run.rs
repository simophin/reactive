use async_broadcast::Receiver;

use crate::{
    effect::{Effect, EffectCleanup},
    effect_context::EffectContext,
    react_context::SignalID,
    task::{Task, TaskHandle},
    tasks_queue::TaskQueueHandle,
    tracker::Tracker,
};

pub struct EffectRun {
    _handle: TaskHandle,
}

impl EffectRun {
    pub fn new(
        task_queue_handle: TaskQueueHandle,
        mut signal_receiver: Receiver<SignalID>,
        mut effect: impl Effect,
    ) -> Self {
        let (task, handle) = {
            let mut task_queue_handle = task_queue_handle.clone();
            Task::new_future(async move {
                let mut tracker = Tracker::default();
                let mut _last_clean_up: AutoEffectCleanUp;

                loop {
                    tracker.clear();
                    Tracker::set_current(Some(tracker));
                    let mut effect_ctx = EffectContext::new(task_queue_handle);
                    _last_clean_up = AutoEffectCleanUp::new(effect.run(&mut effect_ctx));
                    task_queue_handle = effect_ctx.task_queue_handle;
                    tracker = Tracker::set_current(None).expect("To have tracker back");

                    // Wait for signal changes
                    loop {
                        match signal_receiver.recv().await {
                            Ok(id) if tracker.contains(id) => break,
                            Ok(_) => {}
                            Err(_) => return,
                        }
                    }
                }
            })
        };

        let _ = task_queue_handle.queue_task(task);

        Self { _handle: handle }
    }
}

impl Drop for EffectRun {
    fn drop(&mut self) {
        log::debug!("EffectRun dropped");
    }
}

struct AutoEffectCleanUp(Option<Box<dyn EffectCleanup>>);

impl AutoEffectCleanUp {
    fn new(clean_up: Box<dyn EffectCleanup>) -> Self {
        Self(Some(clean_up))
    }
}

impl Drop for AutoEffectCleanUp {
    fn drop(&mut self) {
        self.0.take().unwrap().cleanup();
    }
}
