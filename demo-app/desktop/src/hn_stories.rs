use reactive_core::{Match, ResourceState, SetupContext, Signal, SignalExt, StoredSignal, extract};
use std::cell::Cell;
use std::rc::Rc;
use ui_core::layout::{CrossAxisAlignment, EdgeInsets, Expanded, Padding, SizedBox};
use ui_core::widgets::{Button, Column, Label, List, Platform, ProgressIndicator, Window};

// ---------------------------------------------------------------------------
// Platform selection
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
type AppPlatform = ui_core::gtk::platform::Gtk;
// type AppPlatform = ui_core::appkit::platform::AppKit;

#[cfg(target_os = "linux")]
type AppPlatform = ui_core::gtk::platform::Gtk;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize, Clone, PartialEq)]
struct Story {
    #[serde(rename = "objectID")]
    id: String,
    title: Option<String>,
    author: Option<String>,
    points: Option<i32>,
    num_comments: Option<i32>,
}

#[derive(serde::Deserialize)]
struct HnResponse {
    hits: Vec<Story>,
}

// ---------------------------------------------------------------------------
// API fetch
// ---------------------------------------------------------------------------

async fn fetch_page(page: u32) -> Vec<Story> {
    let url = format!(
        "https://hn.algolia.com/api/v1/search_by_date?tags=story&page={}",
        page
    );
    async {
        reqwest::Client::new()
            .get(&url)
            .send()
            .await?
            .json::<HnResponse>()
            .await
            .map(|r| r.hits)
    }
    .await
    .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

fn app<P: Platform>(ctx: &mut SetupContext)
where
    P::List: 'static,
{
    // `page` drives `page_result`. Incrementing it loads the next page.
    let page: StoredSignal<u32> = ctx.create_signal(0);
    let page_result = ctx.create_resource(page.clone(), fetch_page);

    // Accumulated list of all stories fetched so far.
    let items: StoredSignal<Vec<Story>> = ctx.create_signal(Vec::new());

    // Non-reactive counter so we can tell whether the current page has already
    // been appended (guards against duplicate appends if the effect re-fires).
    let last_appended: Rc<Cell<Option<u32>>> = Rc::new(Cell::new(None));

    {
        let items = items.clone();
        let page = page.clone();
        let last_appended = Rc::clone(&last_appended);
        let page_result_for_effect = page_result.clone();
        ctx.create_effect(move |_, _: Option<()>| {
            let current_page = page.read();
            if let ResourceState::Ready(new_stories) = page_result_for_effect.read() {
                if last_appended.get() != Some(current_page) {
                    last_appended.set(Some(current_page));
                    items.update_with(|existing| {
                        existing.extend(new_stories);
                        true
                    });
                }
            }
        });
    }

    ctx.child(Padding {
        insets: EdgeInsets::all(12),
        child: P::Column::new()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(P::Label::new("Hacker News — Latest Stories").font_size(18.0))
            .child(Expanded {
                flex: std::num::NonZeroUsize::new(1),
                child: P::List::new(items.clone(), |story| Padding {
                    insets: EdgeInsets::all(8),
                    child: P::Column::new()
                        .spacing(2usize)
                        .cross_axis_alignment(CrossAxisAlignment::Stretch)
                        .child(P::Label::new(
                            story
                                .clone()
                                .map_value(|s| s.title.unwrap_or_else(|| "(no title)".into())),
                        ))
                        .child(
                            P::Label::new(story.map_value(|s| {
                                format!(
                                    "by {} · {} pts · {} comments",
                                    s.author.as_deref().unwrap_or("?"),
                                    s.points.unwrap_or(0),
                                    s.num_comments.unwrap_or(0),
                                )
                            }))
                            .font_size(11.0),
                        ),
                }),
            })
            .child(SizedBox::height(44usize).child(
                Match::new(page_result, || P::ProgressIndicator::new_spinner()).case(
                    extract!(ResourceState::Ready(_v) => ()),
                    move |_| {
                        P::Button::new("Load More", {
                            let page = page.clone();
                            move || {
                                page.update_with(|p| {
                                    *p += 1;
                                    true
                                });
                            }
                        })
                    },
                ),
            )),
    });
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

    AppPlatform::run_app(|ctx| {
        ctx.child(<AppPlatform as Platform>::Window::new(
            "HN Stories",
            app::<AppPlatform>,
            800.0,
            700.0,
        ));
    });
}
