use appkit::{Window, run_app};
use demo_app::{AppState, setup};
use reactive_core::{ResourceState, SignalExt};
use reactive_core::signal::IntoSignal;
use ui::button::{Button, ButtonComponent};
use ui::layout::{Column, ColumnComponent, Row, RowComponent};
use ui::text::{Text, TextComponent};

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

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(2)
        .build()
        .unwrap();
    let _guard = rt.enter();

    run_app(|ctx| {
        let AppState { count } = setup(ctx);
        let fact = ctx.create_resource(count, fetch_cat_fact);

        ctx.child(
            Window::new("Reactive Demo".to_string().into_signal(), 520.0, 260.0).child(
                <Column as ColumnComponent>::new()
                    .child(Text::new(count.map(|c| format!("Count: {c}"))))
                    .child(
                        <Row as RowComponent>::new()
                            .child(Button::new("−".to_string(), {
                                let count = count.clone();
                                move || count.update(|v| { *v -= 1; true })
                            }))
                            .child(Button::new("+".to_string(), {
                                let count = count.clone();
                                move || count.update(|v| { *v += 1; true })
                            }))
                            .child(Button::new("Reset".to_string(), move || {
                                count.set_and_notify_changes(0)
                            })),
                    )
                    .child(Text::new(fact.map(|state| match state {
                        ResourceState::Loading(last) => last
                            .map(|s| format!("… {s}"))
                            .unwrap_or_else(|| "…".to_string()),
                        ResourceState::Ready(s) => s,
                    }))),
            ),
        );
    });
}
