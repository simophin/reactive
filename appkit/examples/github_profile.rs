use appkit::stack::PROP_SPACING;
use appkit::text::PROP_FONT_SIZE;
use appkit::{BindableView, ProgressIndicator, Stack, Text, TextInput, Window, run_app};
use reactive_core::components::Show;
use reactive_core::{BoxedComponent, ResourceState, Signal};
use serde::Deserialize;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Deserialize, Clone, Debug)]
struct GitHubUser {
    login: String,
    name: Option<String>,
    bio: Option<String>,
    public_repos: u64,
    followers: u64,
    following: u64,
    location: Option<String>,
}

#[derive(Clone, Debug)]
enum ProfileState {
    Idle,
    Found(GitHubUser),
    NotFound,
    Error(String),
}

// ---------------------------------------------------------------------------
// Network
// ---------------------------------------------------------------------------

async fn fetch_user(username: String) -> ProfileState {
    if username.is_empty() {
        return ProfileState::Idle;
    }

    let client = reqwest::Client::new();
    let resp = match client
        .get(format!("https://api.github.com/users/{username}"))
        .header("User-Agent", "reactive-appkit-example")
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return ProfileState::Error(e.to_string()),
    };

    match resp.status().as_u16() {
        404 => ProfileState::NotFound,
        200..=299 => match resp.json::<GitHubUser>().await {
            Ok(user) => ProfileState::Found(user),
            Err(e) => ProfileState::Error(e.to_string()),
        },
        code => ProfileState::Error(format!("HTTP {code}")),
    }
}

// ---------------------------------------------------------------------------
// UI helpers
// ---------------------------------------------------------------------------

/// A labelled row for the profile table.
fn info_row(label: &'static str, value: String) -> impl reactive_core::Component {
    Stack::new_horizontal_stack()
        .bind(PROP_SPACING, 8.0)
        .child(Text::new_text(label).bind(PROP_FONT_SIZE, 13.0))
        .child(Text::new_text(value).bind(PROP_FONT_SIZE, 13.0))
}

fn hint(text: &'static str) -> impl reactive_core::Component {
    Text::new_text(text).bind(PROP_FONT_SIZE, 13.0)
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    // Reqwest needs a multi-thread Tokio runtime with I/O enabled.
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(2)
        .build()
        .unwrap();
    let _guard = rt.enter();

    run_app(|ctx| {
        // The submitted query drives the resource. Starts empty so the initial
        // resource result is ProfileState::Idle with no network call.
        let search_query = ctx.create_signal(String::new());

        let profile =
            ctx.create_resource(search_query.clone(), |q| async move { fetch_user(q).await });

        // Condition signals — plain closures are Signal<Value = bool>.
        let is_loading = {
            let profile = profile.clone();
            let sq = search_query.clone();
            // Only show the spinner when there is actually a pending request.
            move || !sq.read().is_empty() && matches!(profile.read(), ResourceState::Loading)
        };

        let is_idle = {
            let profile = profile.clone();
            let sq = search_query.clone();
            move || {
                sq.read().is_empty()
                    || matches!(profile.read(), ResourceState::Ready(ProfileState::Idle))
            }
        };

        let is_not_found = {
            let profile = profile.clone();
            move || matches!(profile.read(), ResourceState::Ready(ProfileState::NotFound))
        };

        let is_error = {
            let profile = profile.clone();
            move || matches!(profile.read(), ResourceState::Ready(ProfileState::Error(_)))
        };

        let is_found = {
            let profile = profile.clone();
            move || matches!(profile.read(), ResourceState::Ready(ProfileState::Found(_)))
        };

        ctx.child(
            Window::new("GitHub Profile Viewer", 480.0, 520.0).child(
                Stack::new_vertical_stack()
                    .bind(PROP_SPACING, 16.0)
                    // ── Title ────────────────────────────────────────────
                    .child(Text::new_text("GitHub Profile Viewer").bind(PROP_FONT_SIZE, 22.0))
                    // ── Search bar ───────────────────────────────────────
                    .child(TextInput::new_text_input(
                        "Enter a GitHub username and press Return…",
                        {
                            let search_query = search_query.clone();
                            move |username| {
                                search_query.set_and_notify_changes(username);
                            }
                        },
                    ))
                    // ── Loading spinner ──────────────────────────────────
                    .child(Show::new(
                        is_loading,
                        || -> BoxedComponent { Box::new(ProgressIndicator::new_spinner()) },
                        || Box::new(()),
                    ))
                    // ── Idle hint ────────────────────────────────────────
                    .child(Show::new(
                        is_idle,
                        || -> BoxedComponent {
                            Box::new(hint("Type a GitHub username above and press Return."))
                        },
                        || Box::new(()),
                    ))
                    // ── Not found ────────────────────────────────────────
                    .child(Show::new(
                        is_not_found,
                        {
                            let sq = search_query.clone();
                            move || -> BoxedComponent {
                                let name = sq.read();
                                Box::new(hint(Box::leak(
                                    format!("User \"{name}\" not found.").into_boxed_str(),
                                )))
                            }
                        },
                        || Box::new(()),
                    ))
                    // ── Error ────────────────────────────────────────────
                    .child(Show::new(
                        is_error,
                        {
                            let profile = profile.clone();
                            move || -> BoxedComponent {
                                let msg = match profile.read() {
                                    ResourceState::Ready(ProfileState::Error(e)) => e,
                                    _ => "Unknown error.".to_string(),
                                };
                                Box::new(Text::new_text(msg).bind(PROP_FONT_SIZE, 13.0))
                            }
                        },
                        || Box::new(()),
                    ))
                    // ── Profile card ─────────────────────────────────────
                    .child(Show::new(
                        is_found,
                        {
                            let profile = profile.clone();
                            move || -> BoxedComponent {
                                let ResourceState::Ready(ProfileState::Found(user)) =
                                    profile.read()
                                else {
                                    return Box::new(());
                                };
                                Box::new(
                                    Stack::new_vertical_stack()
                                        .bind(PROP_SPACING, 6.0)
                                        .child(info_row("Login", user.login))
                                        .child(info_row("Name", user.name.unwrap_or_default()))
                                        .child(info_row("Bio", user.bio.unwrap_or_default()))
                                        .child(info_row(
                                            "Location",
                                            user.location.unwrap_or_default(),
                                        ))
                                        .child(info_row(
                                            "Public repos",
                                            user.public_repos.to_string(),
                                        ))
                                        .child(info_row("Followers", user.followers.to_string()))
                                        .child(info_row("Following", user.following.to_string())),
                                )
                            }
                        },
                        || Box::new(()),
                    )),
            ),
        );
    });
}
