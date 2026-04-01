use appkit::platform::AppKit;
use appkit::run_app;
use reactive_core::{Match, ResourceState, SetupContext, Signal, SignalExt, extract};
use resources::ResourceContext;
use resources::reactive::{provide_resource_context, use_resource_context};
use std::num::NonZeroUsize;
use ui_core::layout::types::TextAlignment;
use ui_core::layout::{Center, CrossAxisAlignment, EdgeInsets, Expanded, Padding, SizedBox};
use ui_core::widgets::{Button, Column, Image, Label, Platform, ProgressIndicator, Row, Window};

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

fn app<P: Platform>(ctx: &mut SetupContext)
where
    P::Image: 'static,
{
    let _resource_context = provide_resource_context(ctx, ResourceContext::default());
    let resource_ctx = use_resource_context(ctx);
    let count = ctx.create_signal(0);
    let fact = ctx.create_resource(count.clone(), fetch_cat_fact);

    let testing_image = resource_ctx
        .resolve_asset(ctx, assets::images::TESTING_RESOURCE)
        .map_value(|data| {
            <<P as Platform>::Image as Image>::NativeHandle::try_from(data.0.to_vec())
                .expect("demo image resource must decode")
        });
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
            .child(SizedBox::squared(96usize).child(P::Image::new(
                testing_image,
                Some("Bundled checkerboard test image".to_string()),
            )))
            .child(
                SizedBox::height(44usize).child(
                    P::Row::new()
                        .spacing(12usize)
                        .cross_axis_alignment(CrossAxisAlignment::Stretch)
                        .child(SizedBox::width(72usize).child(P::Button::new("-", {
                            let count = count.clone();
                            move || count.update_if_changes(count.read() - 1)
                        })))
                        .child(Expanded {
                            flex: shared_flex,
                            child: Center {
                                child: P::Label::new(
                                    count.clone().map_value(|c| format!("Count: {c}")),
                                )
                                .font_size(18.0),
                            },
                        })
                        .child(SizedBox::width(72usize).child(P::Button::new("+", {
                            let count = count.clone();
                            move || count.update_if_changes(count.read() + 1)
                        })))
                        .child(
                            SizedBox::width(92usize)
                                .child(P::Button::new("Reset", move || count.update_if_changes(0))),
                        ),
                ),
            )
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
