use appkit::{run_app, Text, Window};
use futures::StreamExt;
use std::time::Duration;

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_time()
        .worker_threads(1)
        .build()
        .unwrap();
    let _guard = rt.enter();

    run_app(|ctx| {
        let tick_stream = tokio_stream::wrappers::IntervalStream::new(
            tokio::time::interval(Duration::from_secs(1)),
        )
        .skip(1)
        .take(100)
        .scan(0i32, |count, _| {
            *count += 1;
            futures::future::ready(Some(*count))
        });

        let count = ctx.create_stream(0, tick_stream);

        // create_memo: derives a String signal from the i32 count signal
        let display = ctx.create_memo(move |ectx| format!("Count: {}", ectx.read(count)));

        ctx.child(
            Window::new("Reactive App", 400.0, 200.0).child(Text::new(display)),
        );
    });
}
