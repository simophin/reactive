use appkit::platform::AppKit;
use appkit::run_app;
use reactive_core::{ResourceState, SetupContext, SignalExt};
use ui_core::widgets::{Button, Column, Label, Platform, Row, Window};

#[derive(serde::Deserialize)]
struct CatFact {
    fact: String,
}

async fn fetch_cat_fact(_count: i32) -> String {
    let result = async {
        let resp = reqwest::Client::new()
            .get("https://catfact.ninja/fact")
            .send()
            .await?;
        Ok::<String, reqwest::Error>(resp.json::<CatFact>().await?.fact)
    }
    .await;
    result.unwrap_or_else(|e| format!("Error: {e}"))
}

fn app<P: Platform>(ctx: &mut SetupContext) {
    let count = ctx.create_signal(0);
    let fact = ctx.create_resource(count.clone(), fetch_cat_fact);

    ctx.child(
        P::Window::new("Reactive Demo", 520.0, 260.0).child(
            P::Column::new()
                .child(P::Label::new(
                    count.clone().map_value(|c| format!("Count: {c}")),
                ))
                .child(
                    P::Row::new()
                        .child(P::Button::new("-", {
                            let count = count.clone();
                            move || {
                                count.update_with(|v| {
                                    *v -= 1;
                                    true
                                })
                            }
                        }))
                        .child(P::Button::new("+", {
                            let count = count.clone();
                            move || {
                                count.update_with(|v| {
                                    *v += 1;
                                    true
                                })
                            }
                        }))
                        .child(P::Button::new("Reset", move || count.update_if_changes(0))),
                )
                .child(P::Label::new(fact.map_value(|state| {
                    match state {
                        ResourceState::Loading(last) => last
                            .map(|s| format!("… {s}"))
                            .unwrap_or_else(|| "…".to_string()),
                        ResourceState::Ready(s) => s,
                    }
                }))),
        ),
    );
}

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(2)
        .build()
        .unwrap();
    let _guard = rt.enter();

    run_app(|ctx| {
        app::<AppKit>(ctx);
    });
}
