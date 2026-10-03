use reactive::prelude::*;

reactive::app!(app);

fn app(ctx: &mut SetupContext) {
    ctx.child(Window::new("{{app_name}}", counter, 400.0, 300.0));
}

fn counter(ctx: &mut SetupContext) {
    let count = ctx.create_signal(0);

    ctx.child(
        Flex::new(
            FlexProps {
                direction: FlexDirection::Column,
                gap: FlexUnit::Absolute(12),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..Default::default()
            }
            .into_signal(),
        )
        .modifier(Modifier::new().paddings(EdgeInsets::all(24)))
        .with_child({
            let count = count.clone();
            move |_| Label::new(count.map_value(|n| format!("Clicked {n} times"))).font_size(20.0)
        })
        .with_child(move |_| Button::new("Click me", move || count.update(count.read() + 1))),
    );
}
