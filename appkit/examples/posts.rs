use appkit::progress_indicator::PROP_INDETERMINATE;
use appkit::stack::PROP_SPACING;
use appkit::text::PROP_FONT_SIZE;
use appkit::{BindableView, CollectionView, ProgressIndicator, Stack, Text, Window, run_app};
use objc2::msg_send;
use objc2_app_kit::NSCollectionViewFlowLayout;
use objc2_foundation::{MainThreadMarker, NSSize};
use reactive_core::{BoxedComponent, Match, ReadSignal, ResourceState, SignalExt, extract};
use serde::Deserialize;
use std::rc::Rc;

// ---------------------------------------------------------------------------
// Data
// ---------------------------------------------------------------------------

#[derive(Deserialize, Clone, PartialEq, Debug)]
struct Post {
    id: u32,
    title: String,
    body: String,
}

// ---------------------------------------------------------------------------
// Network
// ---------------------------------------------------------------------------

async fn fetch_posts() -> Vec<Post> {
    match reqwest::get("https://jsonplaceholder.typicode.com/posts").await {
        Ok(resp) => resp.json::<Vec<Post>>().await.unwrap_or_default(),
        Err(e) => {
            eprintln!("fetch error: {e}");
            Vec::new()
        }
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(2)
        .build()
        .unwrap();
    let _guard = rt.enter();

    run_app(|ctx| {
        let mtm = MainThreadMarker::new().expect("must run on main thread");

        // Fixed-row flow layout.  Each item spans the full content width so
        // the collection behaves as a plain vertical list.
        let flow = NSCollectionViewFlowLayout::new(mtm);
        unsafe {
            let _: () = msg_send![&*flow, setItemSize: NSSize::new(580.0, 64.0)];
            let _: () = msg_send![&*flow, setMinimumLineSpacing: 1.0_f64];
        }
        let layout = flow.into_super();

        // Fire the network request as a resource.  While loading the signal
        // holds `ResourceState::Loading`; once the future resolves it becomes
        // `ResourceState::Ready(Vec<Post>)`.
        let posts = ctx.create_resource((), |_| fetch_posts());

        ctx.boxed_child(
            Window::new("Posts", 620.0, 800.0).child(
                // Show a spinner while loading; switch to the list when ready.
                Match::new(posts, || -> BoxedComponent {
                    Box::new(ProgressIndicator::new_bar(0.0).bind(PROP_INDETERMINATE, true))
                })
                .case(
                    // Extract the Vec<Post> when the resource is ready.
                    extract!(ResourceState::Ready(v) => std::mem::take(v).into()),
                    // Factory: called once when the case first activates.
                    // `items` is a ReadSignal<Rc<[Post]>> that stays current.
                    move |items: ReadSignal<Rc<[Post]>>| -> BoxedComponent {
                        Box::new(CollectionView::new(
                            items,
                            // Cell builder: called once per allocated cell; the
                            // signal is updated in-place on reuse.
                            |post: ReadSignal<Post>| {
                                Stack::new_vertical_stack()
                                    .bind(PROP_SPACING, 3.0)
                                    .child(
                                        Text::new_text(
                                            post.clone()
                                                .map(|p| format!("#{} — {}", p.id, p.title)),
                                        )
                                        .bind(PROP_FONT_SIZE, 13.0),
                                    )
                                    .child(
                                        Text::new_text(post.map(|p| {
                                            p.body.lines().next().unwrap_or("").to_string()
                                        }))
                                        .bind(PROP_FONT_SIZE, 11.0),
                                    )
                            },
                            // Clone the layout handle — Retained<T> is cheap to
                            // clone (it just increments the ObjC retain count).
                            layout.clone(),
                        ))
                    },
                ),
            ),
        );
    });
}
