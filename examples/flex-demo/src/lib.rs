use reactive::prelude::*;

reactive::app!(app);

fn app(ctx: &mut SetupContext) {
    ctx.child(Window::new("ui-core flex demo", flex_demo, 560.0, 360.0));
}

fn flex_demo(ctx: &mut SetupContext) {
    let root_props = ctx.create_signal(FlexProps {
        direction: FlexDirection::Row,
        wrap: FlexWrap::Wrap,
        gap: FlexUnit::Percent(1),
        justify_content: JustifyContent::Start,
        align_items: AlignItems::Center,
        align_content: AlignContent::Start,
    });

    let font_size = ctx.create_signal(15.0);

    {
        let font_size = font_size.clone();
        ctx.create_resource(font_size.clone(), move |size| {
            let font_size = font_size.clone();
            async move {
                tokio::time::sleep(std::time::Duration::from_secs(1000)).await;
                font_size.update(if size < 30.0 { size + 1.0 } else { size });
            }
        });
    }

    let show_first_line = ctx.create_signal(true);

    ctx.child(
        Flex::new(root_props.clone())
            .modifier(Modifier::new().paddings(EdgeInsets::all(16)))
            .with_child(|_| {
                let badge = ImageCodec::decode_static(include_bytes!("../assets/badge.png"))
                    .expect("decode badge");
                Image::new(badge.into_signal(), Some("Badge"))
            })
            .with_child(|flex| {
                Label::new("Flex layout")
                    .font_size(font_size)
                    .alignment(TextAlignment::Leading.into_signal())
                    .modifier(
                        Modifier::new()
                            .with(flex.flex_basis(), FlexUnit::Absolute(480).into_signal())
                            .with(flex.flex_grow(), 1.0_f32)
                            .with(flex.flex_shrink(), 1.0_f32),
                    )
            })
            .with_child({
                let show_first_line = show_first_line.clone();
                move |flex| {
                    Show::new(
                        move || show_first_line.read(),
                        move || {
                            Box::new(Label::new(
                                "Items wrap as the window narrows, with a fixed gap between rows and columns.",
                            )
                                .font_size(14.0)
                                .alignment(TextAlignment::Leading.into_signal())
                                .modifier(
                                    Modifier::new()
                                        .with(flex.flex_grow(), 1.0_f32)
                                        .with(flex.flex_shrink(), 1.0_f32),
                                ))
                        },
                        || Box::new(()),
                    )
                }
            })
            .with_child(move |_| {
                Button::new(show_first_line.clone().map_value(|v| if v {
                    "Hide text".into()
                } else {
                    "Show text".into()
                }), move || {
                    println!("Primary clicked");
                    show_first_line.update(!show_first_line.read());
                })
            })
            .with_child(move |_| {
                Button::new("Add gap", move || {
                    println!("Secondary clicked");
                    let mut props = root_props.read();
                    match &mut props.gap {
                        FlexUnit::Absolute(gap) => *gap += 4,
                        FlexUnit::Percent(gap) => *gap += 1,
                    }

                    root_props.update_if_changes(props);
                })
            }),
    );
}
