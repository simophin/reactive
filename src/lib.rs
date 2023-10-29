mod clean_up;
mod component;
mod effect;
mod node;
mod registry;
mod render;
mod signal;
mod tracker;
mod util;

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
        effect::create_effect,
        render::RenderContext,
        signal::{create_signal, Signal},
    };

    pub fn app() -> impl Component {
        let (title, set_title) = create_signal("hello_world");
        let (body, set_body) = create_signal("body");

        create_effect(move || {
            let set_title = set_title.clone();
            let set_body = set_body.clone();
            spawn_local(async move {
                let mut id = 0;
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    set_title.set(format!("hello_world_{}", id));
                    set_body.set(format!("body_{}", id));
                    id += 1;
                }
            });
        });

        on_clean_up(|| {
            println!("app clean up");
        });

        vec![
            boxed_component(move || header(title.clone())),
            boxed_component(move || content(body.clone())),
        ]
    }

    pub fn header(title: Signal<String>) {
        create_effect(move || {
            println!("Title: {}", title.get());
        });

        on_clean_up(|| {
            println!("header clean up");
        });
    }

    pub fn content(body: Signal<String>) {
        create_effect(move || {
            println!("Body: {}", body.get());
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
