use appkit::{Button, HStack, PROP_ENABLED, Text, VStack, Window, run_app};
use reactive_core::IntoSignal;
use reactive_core::ext::SignalExt;

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_time()
        .worker_threads(1)
        .build()
        .unwrap();
    let _guard = rt.enter();

    run_app(|ctx| {
        let count = ctx.create_signal(0i32);

        ctx.child(
            Window::new("Reactive App", 400.0, 300.0).child(
                VStack::new()
                    .spacing(16.0)
                    .child(Text::new(count.clone().map(|c| format!("Count: {c}"))).font_size(24.0))
                    .child(
                        HStack::new()
                            .spacing(8.0)
                            .child(Button::new("−".into_signal().map(|s| s.to_string()), {
                                let count = count.clone();
                                move || {
                                    count.update(|v| {
                                        *v -= 1;
                                        true
                                    })
                                }
                            }))
                            .child(
                                Button::new(count.clone().map(|c| format!("+ Count: {c}")), {
                                    let count = count.clone();
                                    move || {
                                        count.update(|v| {
                                            *v += 1;
                                            true
                                        })
                                    }
                                })
                                .bind(PROP_ENABLED, count.clone().map(|c| c % 2 == 0)),
                            )
                            .child(Button::new("Reset".to_string().into_signal(), {
                                let count = count.clone();
                                move || count.set_and_notify_changes(0)
                            })),
                    ),
            ),
        );
    });
}
