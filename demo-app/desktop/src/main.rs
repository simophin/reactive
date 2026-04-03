use futures::FutureExt;
use reactive_core::{Match, ResourceState, SetupContext, Signal, SignalExt, extract};
use resources::ResourceContext;
use resources::reactive::{provide_resource_context, use_resource_context};
use std::num::NonZeroUsize;
use tokio::task::spawn_blocking;
use ui_core::layout::types::TextAlignment;
use ui_core::layout::{Center, CrossAxisAlignment, EdgeInsets, Expanded, Padding, SizedBox};
use ui_core::widgets::{
    Button, Column, Image, ImageCodec, Label, Platform, ProgressIndicator, Row, Window,
};

include!(concat!(env!("OUT_DIR"), "/resources.rs"));

// Platform selection: the only two lines that differ between macOS and Linux.
// #[cfg(target_os = "macos")]
// type AppPlatform = appkit::platform::AppKit;
// #[cfg(target_os = "linux")]
type AppPlatform = gtk::platform::Gtk;

// ---------------------------------------------------------------------------
// Application UI — generic over any Platform, compiled once.
// ---------------------------------------------------------------------------

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
    let _resource_context = provide_resource_context(ctx, ResourceContext::default());
    let resource_ctx = use_resource_context(ctx);
    let count = ctx.create_signal(0);
    let fact = ctx.create_resource(count.clone(), fetch_cat_fact);

    let testing_image = resource_ctx.resolve_asset(ctx, assets::images::TESTING_RESOURCE);
    let testing_image = ctx.create_resource(testing_image, move |input| {
        spawn_blocking(move || P::ImageCodec::decode_static(input.data())).map(|r| match r {
            Ok(Ok(image)) => Ok(image),
            Ok(Err(e)) => Err(format!("Error decoding image: {e:?}")),
            Err(_) => Err(String::from("Error joining future")),
        })
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
            .child(
                SizedBox::squared(96usize).child(
                    Match::new(testing_image, || P::Label::new("Loading image..."))
                        .case(extract!(ResourceState::Ready(Ok(img)) => img), |img| {
                            P::Image::new(img, Some("Bundled checkerboard test image"))
                        })
                        .case(extract!(ResourceState::Ready(Err(err)) => err), |err| {
                            P::Label::new(err)
                        }),
                ),
            )
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
                    .case(extract!(ResourceState::Ready(v) => v), P::Label::new),
                },
            }),
    });
}

fn main() {
    let _ = dotenvy::dotenv();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(2)
        .build()
        .unwrap();
    let _guard = rt.enter();

    AppPlatform::run_app(|ctx| {
        ctx.child(<AppPlatform as Platform>::Window::new(
            "Reactive Demo",
            app::<AppPlatform>,
            600.0,
            600.0,
        ));
    });
}
