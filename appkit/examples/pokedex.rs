use appkit::progress_indicator::PROP_INDETERMINATE;
use appkit::stack::PROP_SPACING;
use appkit::text_view::PROP_SELECTABLE;
use appkit::{BindableView, PROP_FONT_SIZE, ProgressIndicator, Stack, TextView, Window, run_app};
use reactive_core::{BoxedComponent, Match, ResourceState, Signal, extract};
use serde::Deserialize;
// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Deserialize, Clone, Debug)]
struct PokemonType {
    #[serde(rename = "type")]
    type_info: TypeName,
}

#[derive(Deserialize, Clone, Debug)]
struct TypeName {
    name: String,
}

#[derive(Deserialize, Clone, Debug)]
struct PokemonAbility {
    ability: AbilityName,
}

#[derive(Deserialize, Clone, Debug)]
struct AbilityName {
    name: String,
}

#[derive(Deserialize, Clone, Debug)]
struct Pokemon {
    id: u64,
    name: String,
    height: u64,
    weight: u64,
    base_experience: Option<u64>,
    types: Vec<PokemonType>,
    abilities: Vec<PokemonAbility>,
}

#[derive(Clone, Debug)]
enum ProfileState {
    Idle,
    Found(Pokemon),
    NotFound,
    Error(String),
}

// ---------------------------------------------------------------------------
// Network
// ---------------------------------------------------------------------------

async fn fetch_pokemon(name: String) -> ProfileState {
    if name.is_empty() {
        return ProfileState::Idle;
    }

    let client = reqwest::Client::new();
    let resp = match client
        .get(format!(
            "https://pokeapi.co/api/v2/pokemon/{}",
            name.to_lowercase()
        ))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return ProfileState::Error(e.to_string()),
    };

    match resp.status().as_u16() {
        404 => ProfileState::NotFound,
        200..=299 => match resp.json::<Pokemon>().await {
            Ok(pokemon) => ProfileState::Found(pokemon),
            Err(e) => ProfileState::Error(e.to_string()),
        },
        code => ProfileState::Error(format!("HTTP {code}")),
    }
}

// ---------------------------------------------------------------------------
// UI helpers
// ---------------------------------------------------------------------------

fn info_row(label: &'static str, value: String) -> impl reactive_core::Component {
    Stack::new_horizontal_stack()
        .bind(PROP_SPACING, 8.0)
        .child(TextView::label(label).bind(PROP_FONT_SIZE, 13.0))
        .child(TextView::label(value).bind(PROP_FONT_SIZE, 13.0))
}

fn hint(text: &'static str) -> impl reactive_core::Component {
    TextView::label(text).bind(PROP_FONT_SIZE, 13.0)
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
        let input_text = ctx.create_signal(String::new());
        let search_query = ctx.create_signal(String::new());

        let profile =
            ctx.create_resource(
                search_query.clone(),
                |q| async move { fetch_pokemon(q).await },
            );

        ctx.boxed_child(
            Window::new("Pokédex", 480.0, 520.0).child(
                Stack::new_vertical_stack()
                    .bind(PROP_SPACING, 16.0)
                    .child(TextView::label("Pokédex")
                        .bind(PROP_SELECTABLE, false)
                        .bind(PROP_FONT_SIZE, 22.0))
                    .child(TextView::input_text(
                        input_text,
                        {
                            let search_query = search_query.clone();
                            move |name| search_query.update_if_changes(name)
                        },
                    ))
                    .child(
                        Match::new(profile, || -> BoxedComponent {
                            Box::new(hint("Type a Pokémon name above and press Return."))
                        })
                        // Loading — show progress bar only when a query is in flight
                        .case(
                            {
                                let search_query = search_query.clone();
                                move |state| match state {
                                    ResourceState::Loading(_)
                                        if !search_query.read().is_empty() =>
                                    {
                                        Some(())
                                    }
                                    _ => None,
                                }
                            },
                            |_sig| -> BoxedComponent {
                                Box::new(
                                    ProgressIndicator::new_bar(0.0)
                                        .bind(PROP_INDETERMINATE, true),
                                )
                            },
                        )
                        // Found — display the Pokémon details
                        .case(
                            extract!(ResourceState::Ready(ProfileState::Found(p)) => p.clone()),
                            |sig| -> BoxedComponent {
                                let p = sig.read();
                                let types = p
                                    .types
                                    .iter()
                                    .map(|t| t.type_info.name.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                let abilities = p
                                    .abilities
                                    .iter()
                                    .map(|a| a.ability.name.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                Box::new(
                                    Stack::new_vertical_stack()
                                        .bind(PROP_SPACING, 6.0)
                                        .child(info_row("Name", p.name.clone()))
                                        .child(info_row("ID", format!("#{}", p.id)))
                                        .child(info_row(
                                            "Height",
                                            format!("{:.1} m", p.height as f64 / 10.0),
                                        ))
                                        .child(info_row(
                                            "Weight",
                                            format!("{:.1} kg", p.weight as f64 / 10.0),
                                        ))
                                        .child(info_row(
                                            "Base exp",
                                            p.base_experience
                                                .map(|e| e.to_string())
                                                .unwrap_or_else(|| "—".to_string()),
                                        ))
                                        .child(info_row("Types", types))
                                        .child(info_row("Abilities", abilities)),
                                )
                            },
                        )
                        // Not found
                        .case(
                            extract!(ResourceState::Ready(ProfileState::NotFound) => ()),
                            {
                                let search_query = search_query.clone();
                                move |_sig| -> BoxedComponent {
                                    let name = search_query.read();
                                    Box::new(hint(Box::leak(
                                        format!("Pokémon \"{name}\" not found.").into_boxed_str(),
                                    )))
                                }
                            },
                        )
                        // Error
                        .case(
                            extract!(ResourceState::Ready(ProfileState::Error(e)) => std::mem::take(e)),
                            |sig| -> BoxedComponent {
                                let msg = sig.read();
                                Box::new(TextView::label(msg).bind(PROP_FONT_SIZE, 13.0))
                            },
                        ),
                    ),
            ),
        );
    });
}
