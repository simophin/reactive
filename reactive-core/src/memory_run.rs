use crate::{task::Task, tracker::Tracker, SetupContext, Signal};

pub fn new_memory_run<T>(
    ctx: &mut SetupContext,
    mut factory: impl FnMut() -> T + 'static,
) -> impl Signal<Value = T>
where
    T: 'static,
{
    let (mut tracker, initial_value) = Tracker::default().with_current(|| factory());
    let (signal_r, mut signal_w) = ctx.create_signal(initial_value);
    let mut subscriber = ctx.data.signal_sender.subscribe();
    subscriber.set_subscribing(tracker.iter());

    let task = Task::new_future(async move {
        while subscriber.next().await.is_some() {
            tracker.clear();
            let result = tracker.with_current(|| factory());
            tracker = result.0;
            let value = result.1;
            signal_w.update(value);
            subscriber.set_subscribing(tracker.iter());
        }
    });

    match ctx.data.queue.queue_task(task) {
        Ok(clean_up) => ctx.on_clean_up(clean_up),
        Err(_) => log::warn!("Reactive system has stopped accepting task"),
    };

    signal_r
}
