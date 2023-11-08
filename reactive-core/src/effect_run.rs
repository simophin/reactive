use crate::{
    effect::Effect, effect_context::EffectContext, task::Task, tracker::Tracker, SetupContext,
};

pub(crate) fn new_effect_run(ctx: &mut SetupContext, mut effect: impl Effect) {
    let task_queue_handle = ctx.data.queue.clone();

    let task = {
        let data = ctx.data.clone();
        Task::new_future(async move {
            let mut tracker = Tracker::default();
            let mut signal_receiver = data.signal_sender.subscribe();
            let mut effect_ctx = EffectContext::new(data);

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
        return;
    };

    ctx.on_clean_up(clean_up);
}
