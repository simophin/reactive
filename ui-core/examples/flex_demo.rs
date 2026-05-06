use reactive_core::{Component, IntoSignal, SetupContext};
use ui_core::widgets::{
    AlignContent, AlignItems, Button, CommonModifiers, EdgeInsets, Flex, FlexDirection, FlexProps,
    FlexUnit, FlexWrap, JustifyContent, Label, Modifier, Platform, TextAlignment, Window,
    WithModifier,
};

fn main() {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt::init();

    #[cfg(feature = "appkit")]
    run::<ui_core::appkit::platform::AppKit>();

    #[cfg(feature = "gtk")]
    run::<ui_core::gtk::platform::Gtk>();
}

fn run<P: Platform>() {
    P::run_app(|ctx| {
        setup_demo::<P>(ctx);
    });
}

fn setup_demo<P: Platform>(ctx: &mut SetupContext) {
    ctx.child(P::Window::new(
        "ui-core flex demo",
        flex_demo::<P>(),
        560.0,
        360.0,
    ));
}

fn flex_demo<P: Platform>() -> impl Component {
    let root_props = FlexProps {
        direction: FlexDirection::Row,
        wrap: FlexWrap::Wrap,
        gap: FlexUnit::Absolute(12),
        justify_content: JustifyContent::Start,
        align_items: AlignItems::Center,
        align_content: AlignContent::Start,
    };

    P::Flex::new(root_props.into_signal())
        .modifier(Modifier::new().paddings(EdgeInsets::all(16)))
        .with_child(|flex| {
            P::Label::new("Flex layout")
                .font_size(22.0)
                .alignment(TextAlignment::Leading.into_signal())
                .modifier(
                    Modifier::new()
                        .with(flex.flex_basis(), FlexUnit::Absolute(480).into_signal())
                        .with(flex.flex_grow(), 1.0_f32)
                        .with(flex.flex_shrink(), 1.0_f32),
                )
        })
        .with_child(|flex| {
            P::Label::new(
                "Items wrap as the window narrows, with a fixed gap between rows and columns.",
            )
            .font_size(14.0)
            .alignment(TextAlignment::Leading.into_signal())
            .modifier(
                Modifier::new()
                    .with(flex.flex_grow(), 1.0_f32)
                    .with(flex.flex_shrink(), 1.0_f32),
            )
        })
        .with_child(|flex| {
            P::Button::new("Primary", || {
                println!("Primary clicked");
            })
            .modifier(
                Modifier::new()
                    .with(flex.flex_basis(), FlexUnit::Absolute(120).into_signal())
                    .with(flex.flex_shrink(), 0.0_f32),
            )
        })
}
