use appkit::{run_app, stop_app};
use futures::StreamExt;
use std::time::Duration;

fn main() {
    // Start a tokio runtime for timers/async. Multi-threaded so the timer
    // reactor runs on a background thread while CFRunLoop owns the main thread.
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_time()
        .worker_threads(1)
        .build()
        .unwrap();
    let _guard = rt.enter();

    run_app(|ctx| {
        // A tokio-backed interval stream: emits a count every second, up to 5
        let tick_stream = tokio_stream::wrappers::IntervalStream::new(
            tokio::time::interval(Duration::from_secs(1)),
        )
        .skip(1) // skip the immediate first tick
        .take(5)
        .scan(0i32, |count, _| {
            *count += 1;
            futures::future::ready(Some(*count))
        });

        let count = ctx.create_stream(0, tick_stream);

        ctx.create_effect(move |ectx, _: Option<&mut ()>| {
            let value = ectx.read(count);
            println!("[effect] count = {value}");

            if value >= 5 {
                println!("[effect] reached 5, stopping app");
                stop_app();
            }
        });

        println!("[setup]  done, entering run loop...");
    });

    println!("[main]   run loop exited");
}
