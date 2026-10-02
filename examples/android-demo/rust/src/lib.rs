//! Demo app for the Compose backend. Build and install it with Gradle from the parent
//! directory: `./gradlew installDebug`.
#![cfg(target_os = "android")]

use futures::channel::mpsc;
use reactive_core::{IntoSignal, SetupContext, Signal, SignalExt};
use std::time::Duration;
use ui_core::android::ui::platform::Android;
use ui_core::android::ui::text_input::TextInput as AndroidTextInput;
use ui_core::widgets::{
    AlignItems, Button, CommonModifiers, EdgeInsets, Flex, FlexDirection, FlexProps, FlexUnit,
    FlexWrap, Label, Modifier, Platform, ProgressIndicator, Slider, TextCommand, TextInput, Window,
    WithModifier,
};

ui_core::android_main!(|ctx| {
    tracing_subscriber::fmt()
        .with_writer(ui_core::android::LogcatWriter::default)
        .with_ansi(false)
        .without_time()
        .try_init()
        .ok();
    demo::<Android>(ctx);
});

fn column(gap: usize) -> FlexProps {
    FlexProps {
        direction: FlexDirection::Column,
        gap: FlexUnit::Absolute(gap),
        align_items: AlignItems::Stretch,
        ..Default::default()
    }
}

fn row(wrap: FlexWrap) -> FlexProps {
    FlexProps {
        direction: FlexDirection::Row,
        wrap,
        gap: FlexUnit::Absolute(8),
        align_items: AlignItems::Center,
        ..Default::default()
    }
}

/// Uses the shared widget API only, apart from the text input, whose text type is
/// platform-specific.
fn demo<P: Platform>(ctx: &mut SetupContext) {
    let count = ctx.create_signal(0usize);
    let text = ctx.create_signal(String::new());
    let progress = ctx.create_signal(30usize);
    let chips = ctx.create_signal(5usize);
    let (commands, command_rx) = mpsc::channel(4);

    // A background thread drives a signal through a stream, exercising cross-thread wakeups.
    let seconds = ctx.create_stream(
        0u64,
        || (),
        |_| {
            let (mut tx, rx) = mpsc::channel(1);
            std::thread::spawn(move || {
                for n in 1.. {
                    std::thread::sleep(Duration::from_secs(1));
                    if tx.try_send(n).is_err() && tx.is_closed() {
                        break;
                    }
                }
            });
            rx
        },
    );

    let content = P::Flex::new(column(12).into_signal())
        .modifier(Modifier::new().paddings(EdgeInsets::all(24)))
        .with_child(|_| P::Label::new("Rust ❤ Compose").font_size(28.0))
        .with_child(|_| {
            P::Label::new(
                seconds.map_value(|s| format!("Running for {s}s, driven by a Rust stream")),
            )
        })
        .with_child({
            let count = count.clone();
            move |_| {
                P::Label::new(count.map_value(|c| format!("Clicked {c} times"))).font_size(18.0)
            }
        })
        .with_child({
            let count = count.clone();
            move |_| {
                let reset_enabled = count.clone().map_value(|c| c > 0);
                P::Flex::new(row(FlexWrap::NoWrap).into_signal())
                    .with_child({
                        let count = count.clone();
                        move |_| P::Button::new("+1", move || count.update(count.read() + 1))
                    })
                    .with_child(move |_| {
                        P::Button::new("Reset", move || count.update(0usize)).enabled(reset_enabled)
                    })
            }
        })
        .with_child({
            let text = text.clone();
            move |_| {
                AndroidTextInput::new()
                    .with_commander(command_rx)
                    .with_on_text_changed(move |s| text.update(s.to_string()))
            }
        })
        .with_child({
            let text = text.clone();
            move |_| {
                P::Label::new(text.map_value(|t| {
                    if t.is_empty() {
                        "Type above; Rust echoes it here".to_string()
                    } else {
                        format!("You typed {} UTF-16 units: {t}", t.encode_utf16().count())
                    }
                }))
            }
        })
        .with_child(move |_| {
            P::Button::new("Set text from Rust", move || {
                let _ = commands
                    .clone()
                    .try_send(TextCommand::SetText("Hello from Rust".into()));
            })
        })
        .with_child({
            let progress = progress.clone();
            move |_| {
                let value = progress.clone();
                P::Slider::new(value, (0..100).into_signal(), move |v| {
                    progress.update_if_changes(v)
                })
            }
        })
        .with_child({
            let progress = progress.clone();
            move |_| P::ProgressIndicator::new_bar(progress)
        })
        .with_child({
            let chips = chips.clone();
            move |_| {
                P::Button::new(
                    chips.clone().map_value(|n| format!("Wrap {n} labels")),
                    move || {
                        chips.update(if chips.read() >= 20 {
                            1
                        } else {
                            chips.read() + 3
                        })
                    },
                )
            }
        })
        .with_child(move |_| {
            let mut wrap = P::Flex::new(row(FlexWrap::Wrap).into_signal());
            for i in 0..20 {
                let chips = chips.clone();
                wrap = wrap.with_child(move |_| {
                    reactive_core::Show::new(
                        move || i < chips.read(),
                        move || Box::new(P::Label::new(format!("label {i}"))),
                        || Box::new(()),
                    )
                });
            }
            wrap
        });

    ctx.child(P::Window::new("Android demo", content, 400.0, 800.0));
}
