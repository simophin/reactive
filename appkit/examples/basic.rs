use appkit::{Text, Window, run_app};
use futures::StreamExt;
use reactive_core::Signal;
use std::time::Duration;

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_time()
        .worker_threads(1)
        .build()
        .unwrap();
    let _guard = rt.enter();

    run_app(|ctx| {
        let count = ctx.create_stream(0, (), |_| {
            tokio_stream::wrappers::IntervalStream::new(tokio::time::interval(Duration::from_secs(
                1,
            )))
            .skip(1)
            .take(100)
            .scan(0i32, |count, _| {
                *count += 1;
                futures::future::ready(Some(*count))
            })
        });

        // create_memo: derives a String signal from the i32 count signal
        ctx.child(
            Window::new("Reactive App", 400.0, 200.0)
                .child(Text::new(count.map(|c| format!("Count: {c}")))),
        );
    });
}
