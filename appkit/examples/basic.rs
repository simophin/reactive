use appkit::button::PROP_ENABLED;
use appkit::stack::PROP_SPACING;
use appkit::text::PROP_FONT_SIZE;
use appkit::{AppKitViewComponent, BindableView, Window, run_app};
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
                AppKitViewComponent::new_vertical_stack()
                    .bind(PROP_SPACING, 16.0)
                    .child(
                        AppKitViewComponent::new_text(count.clone().map(|c| format!("Count: {c}")))
                            .bind(PROP_FONT_SIZE, 24.0),
                    )
                    .child(
                        AppKitViewComponent::new_horizontal_stack()
                            .bind(PROP_SPACING, 8.0)
                            .child(AppKitViewComponent::new_button("−", {
                                let count = count.clone();
                                move || {
                                    count.update(|v| {
                                        *v -= 1;
                                        true
                                    })
                                }
                            }))
                            .child(
                                AppKitViewComponent::new_button(
                                    count.clone().map(|c| format!("+ Count: {c}")),
                                    {
                                        let count = count.clone();
                                        move || {
                                            count.update(|v| {
                                                *v += 1;
                                                true
                                            })
                                        }
                                    },
                                )
                                .bind(PROP_ENABLED, count.clone().map(|c| c % 2 == 0)),
                            )
                            .child(AppKitViewComponent::new_button("Reset", {
                                let count = count.clone();
                                move || count.set_and_notify_changes(0)
                            })),
                    ),
            ),
        );
    });
}
