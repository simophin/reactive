mod clean_up;
mod component;
mod core_component;
mod effect;
mod node_ref;
mod registry;
mod render;
mod signal;
mod tracker;
mod util;
mod setup;
mod node;
mod react_context;
mod task;

#[cfg(test)]
mod tests {
    use std::{future::pending, time::Duration};

    use tokio::{
        task::{spawn_local, LocalSet},
        time::sleep,
    };

    use crate::{
        clean_up::on_clean_up,
        component::{boxed_component, Component},
        core_component::Show,
        effect::create_effect,
        render::RenderContext,
        signal::{create_signal, Signal},
    };

    pub fn app() -> impl Component {
        let (index, set_index) = create_signal(1usize);

        let title = {
            let index = index.clone();
            move || format!("hello_world_{}", index.get())
        };

        let body = {
            let index = index.clone();
            move || format!("body_{}", index.clone().get())
        };

        create_effect(move || {
            let set_index = set_index.clone();
            spawn_local(async move {
                let mut id = 0usize;
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    set_index.set(id);
                    id += 1;
                }
            });
        });

        on_clean_up(|| {
            println!("app clean up");
        });

        let show = Show::new(
            move || index.get() % 2 == 0,
            move || content(body.clone()),
            (),
        );

        vec![
            boxed_component(move || header(title.clone())),
            boxed_component(show),
        ]
    }

    pub fn header(title: impl Signal<Value = String>) {
        create_effect(move || {
            println!("Title: {}", title.get());
        });

        on_clean_up(|| {
            println!("header clean up");
        });
    }

    pub fn content(body: impl Signal<Value = String>) {
        create_effect(move || {
            println!("content: {}", body.get());

            || println!("content effect clean up")
        });

        on_clean_up(|| {
            println!("content clean up");
        });
    }

    #[tokio::test]
    async fn reactive_works() {
        let set = LocalSet::new();
        set.run_until(async move {
            let mounted = RenderContext::new(Box::new(app)).mount();

            sleep(Duration::from_secs(5)).await;

            mounted.unmount();
            pending::<()>().await;
        })
        .await;
    }
}
