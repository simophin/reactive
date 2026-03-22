use appkit::progress_indicator::PROP_INDETERMINATE;
use appkit::stack::PROP_SPACING;
use appkit::text::PROP_FONT_SIZE;
use appkit::{BindableView, ProgressIndicator, Stack, Text, TextInput, Window, run_app};
use reactive_core::components::Switch;
use reactive_core::{BoxedComponent, ResourceState, Signal};
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
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(2)
        .build()
        .unwrap();
    let _guard = rt.enter();

    run_app(|ctx| {
        let search_query = ctx.create_signal(String::new());

        let profile = ctx.create_resource(search_query, |q| async move {
            fetch_pokemon(q).await
        });

        ctx.child(
            Window::new("Pokédex", 480.0, 520.0).child(
                Stack::new_vertical_stack()
                    .bind(PROP_SPACING, 16.0)
                    .child(Text::new_text("Pokédex").bind(PROP_FONT_SIZE, 22.0))
                    .child(TextInput::new_text_input(
                        "Enter a Pokémon name and press Return…",
                        move |name| search_query.set_and_notify_changes(name),
                    ))
                    .child(
                        Switch::new(|| -> BoxedComponent {
                            Box::new(hint("Type a Pokémon name above and press Return."))
                        })
                        // Loading — indeterminate progress bar
                        .case(
                            move || {
                                !search_query.read().is_empty()
                                    && matches!(profile.read(), ResourceState::Loading)
                            },
                            || -> BoxedComponent {
                                Box::new(
                                    ProgressIndicator::new_bar(0.0)
                                        .bind(PROP_INDETERMINATE, true),
                                )
                            },
                        )
                        // Found
                        .case(
                            move || {
                                matches!(
                                    profile.read(),
                                    ResourceState::Ready(ProfileState::Found(_))
                                )
                            },
                            move || -> BoxedComponent {
                                let ResourceState::Ready(ProfileState::Found(p)) =
                                    profile.read()
                                else {
                                    return Box::new(());
                                };
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
                            move || {
                                matches!(
                                    profile.read(),
                                    ResourceState::Ready(ProfileState::NotFound)
                                )
                            },
                            move || -> BoxedComponent {
                                let name = search_query.read();
                                Box::new(hint(Box::leak(
                                    format!("Pokémon \"{name}\" not found.").into_boxed_str(),
                                )))
                            },
                        )
                        // Error
                        .case(
                            move || {
                                matches!(
                                    profile.read(),
                                    ResourceState::Ready(ProfileState::Error(_))
                                )
                            },
                            move || -> BoxedComponent {
                                let msg = match profile.read() {
                                    ResourceState::Ready(ProfileState::Error(e)) => e,
                                    _ => "Unknown error.".to_string(),
                                };
                                Box::new(Text::new_text(msg).bind(PROP_FONT_SIZE, 13.0))
                            },
                        )
                    ),
            ),
        );
    });
}
