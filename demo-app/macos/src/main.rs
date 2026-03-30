use appkit::platform::AppKit;
use appkit::run_app;
use reactive_core::{Match, ResourceState, SetupContext, SignalExt, extract};
use std::num::NonZeroUsize;
use ui_core::layout::types::TextAlignment;
use ui_core::layout::{Center, CrossAxisAlignment, EdgeInsets, Expanded, Padding, SizedBox};
use ui_core::widgets::{Button, Column, Label, Platform, ProgressIndicator, Row, Window};

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
    let shared_flex = NonZeroUsize::new(1);

    ctx.child(Padding {
        insets: EdgeInsets::all(24),
        child: P::Column::new()
            .spacing(18usize)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(
                P::Label::new("Reactive Demo")
                    .font_size(24.0)
                    .alignment(TextAlignment::Center),
            )
            .child(P::Label::new(
                "The controls below now run through Padding, Center, SizedBox, and Expanded.",
            ))
            .child(SizedBox {
                width: None::<usize>,
                height: Some(44usize),
                child: P::Row::new()
                    .spacing(12usize)
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .child(SizedBox {
                        width: Some(72usize),
                        height: None::<usize>,
                        child: P::Button::new("-", {
                            let count = count.clone();
                            move || {
                                count.update_with(|v| {
                                    *v -= 1;
                                    true
                                })
                            }
                        }),
                    })
                    .child(Expanded {
                        flex: shared_flex,
                        child: Center {
                            child: P::Label::new(
                                count.clone().map_value(|c| format!("Count: {c}")),
                            )
                            .font_size(18.0),
                        },
                    })
                    .child(SizedBox {
                        width: Some(72usize),
                        height: None::<usize>,
                        child: P::Button::new("+", {
                            let count = count.clone();
                            move || {
                                count.update_with(|v| {
                                    *v += 1;
                                    true
                                })
                            }
                        }),
                    })
                    .child(SizedBox {
                        width: Some(92usize),
                        height: None::<usize>,
                        child: P::Button::new("Reset", move || count.update_if_changes(0)),
                    }),
            })
            .child(Expanded {
                flex: shared_flex,
                child: Padding {
                    insets: EdgeInsets::all(16),
                    child: Match::new(fact, || Center {
                        child: P::ProgressIndicator::new_spinner(),
                    })
                    .case(
                        extract!(ResourceState::Ready(v) => std::mem::take(v)),
                        P::Label::new,
                    ),
                },
            }),
    });
}

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(2)
        .build()
        .unwrap();
    let _guard = rt.enter();

    run_app(|ctx| {
        ctx.child(appkit::window::Window::new(
            "Reactive Demo",
            app::<AppKit>,
            500.0,
            500.0,
        ));
    });
}
