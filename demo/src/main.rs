use appkit::{Window, run_app};
use reactive_core::{ResourceState, Signal, SignalExt};
use resources::{LanguageIdentifier, ResourceContext};
use ui::button::{Button, ButtonComponent};
use ui::layout::{Column, ColumnComponent, Row, RowComponent};
use ui::text::{Text, TextComponent};

include!(concat!(env!("OUT_DIR"), "/resources.rs"));

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
        let rc_signal = resources::reactive::provide_resource_context(
            ctx,
            ResourceContext {
                locale: "en-US".parse().unwrap(),
                ..ResourceContext::default()
            },
        );
        let rc = resources::reactive::use_resource_context(ctx);

        let count = ctx.create_signal(0i32);
        let fact = ctx.create_resource(count, fetch_cat_fact);

        let title = rc.translate(ctx, strings::APP_TITLE, ());
        let decr_label = rc.translate(ctx, strings::DECREMENT, ());
        let incr_label = rc.translate(ctx, strings::INCREMENT, ());
        let reset_label = rc.translate(ctx, strings::RESET, ());
        let locale_label = rc.translate(ctx, strings::SWITCH_LOCALE, ());
        let count_label =
            rc.translate_with(ctx, strings::COUNTER_LABEL, move || strings::CounterLabel {
                count: count.read().to_string(),
            });

        ctx.child(
            Window::new(title, 520.0, 260.0).child(
                <Column as ColumnComponent>::new()
                    .child(Text::new(count_label))
                    .child(
                        <Row as RowComponent>::new()
                            .child(Button::new(decr_label, {
                                let count = count.clone();
                                move || {
                                    count.update(|v| {
                                        *v -= 1;
                                        true
                                    })
                                }
                            }))
                            .child(Button::new(incr_label, {
                                let count = count.clone();
                                move || {
                                    count.update(|v| {
                                        *v += 1;
                                        true
                                    })
                                }
                            }))
                            .child(Button::new(reset_label, move || {
                                count.set_and_notify_changes(0)
                            }))
                            .child(Button::new(locale_label, move || {
                                rc_signal.update(|c| {
                                    c.locale = if c.locale.to_string() == "en-US" {
                                        "fr-FR"
                                    } else {
                                        "en-US"
                                    }
                                    .parse::<LanguageIdentifier>()
                                    .unwrap();
                                    true
                                });
                            })),
                    )
                    .child(Text::new(fact.map(|state| {
                        match state {
                            ResourceState::Loading(last) => last
                                .map(|s| format!("… {s}"))
                                .unwrap_or_else(|| "…".to_string()),
                            ResourceState::Ready(s) => s,
                        }
                    }))),
            ),
        );
    });
}
