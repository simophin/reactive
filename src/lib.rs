mod clean_up;
mod component;
mod core_component;
mod effect;
mod effect_context;
mod node;
mod react_context;
mod render;
mod setup_context;
mod signal;
mod task;
mod tracker;
mod util;

#[cfg(test)]
mod tests {
    use std::future::pending;

    use tokio::task::{spawn_local, LocalSet};

    use crate::{
        component::{boxed_component, Component},
        core_component::Show,
        effect_context::EffectContext,
        render::RenderContext,
        setup_context::SetupContext,
        signal::Signal,
    };

    pub fn app(ctx: &mut SetupContext) -> impl Component {
        let (index, set_index) = ctx.create_signal(1usize);

        let title = {
            let index = index.clone();
            move || format!("hello_world_{}", index.get())
        };

        let body = {
            let index = index.clone();
            move || format!("body_{}", index.clone().get())
        };

        ctx.create_effect(move |_: &mut _| {
            let mut set_index = set_index.clone();
            spawn_local(async move {
                let mut id = 0usize;
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    set_index.set(id);
                    id += 1;
                }
            });
        });

        ctx.on_clean_up(|| {
            println!("app clean up");
        });

        let show = Show::new(
            move || index.get() % 2 == 0,
            move || {
                let body = body.clone();
                move |ctx: &mut SetupContext| content(ctx, body.clone())
            },
            || (),
        );

        vec![
            boxed_component(move |ctx: &mut SetupContext| header(ctx, title.clone())),
            boxed_component(show),
        ]
    }

    pub fn header(ctx: &mut SetupContext, title: impl Signal<Value = String>) {
        ctx.create_effect(move |_: &mut EffectContext| {
            println!("Title: {}", title.get());
        });

        ctx.on_clean_up(|| {
            println!("header clean up");
        });
    }

    pub fn content(ctx: &mut SetupContext, body: impl Signal<Value = String>) {
        ctx.create_effect(move |ctx: &mut _| {
            println!("content: {}", body.get());

            || println!("content effect clean up")
        });

        ctx.on_clean_up(|| {
            println!("content clean up");
        });
    }

    #[tokio::test]
    async fn reactive_works() {
        let set = LocalSet::new();
        set.run_until(async move {
            let mut mounted = RenderContext::new(Box::new(app)).setup().mount();

            mounted.wait().await;

            mounted.unmount();
            pending::<()>().await;
        })
        .await;
    }
}
