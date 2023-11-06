use reactive_core::{
    boxed_component,
    core_component::{CaseBuilder, ShowBuilder, SwitchBuilder},
    Component, ReactiveContext, SetupContext, Signal, SignalGetter,
};
use tokio::task::LocalSet;

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

    ctx.create_effect(move |ctx| {
        let set_index = set_index.clone();
        ctx.spawn(async move {
            let mut set_index = set_index.clone();

            loop {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                set_index.update_with(|v| {
                    *v = *v + 1;
                    true
                });
            }
        });
    });

    ctx.on_clean_up(|| {
        println!("app clean up");
    });

    let show = ShowBuilder::default()
        .test(move || index.get() % 2 == 0)
        .success(move || {
            let body = body.clone();
            move |ctx: &mut SetupContext| content(ctx, body.clone())
        })
        .fail(|| ())
        .build()
        .unwrap();

    vec![
        boxed_component(move |ctx: &mut SetupContext| header(ctx, title.clone())),
        boxed_component(show),
    ]
}

pub fn header(ctx: &mut SetupContext, title: impl Signal<Value = String>) -> impl Component {
    ctx.on_clean_up(|| {
        println!("header clean up");
    });

    SwitchBuilder::default()
        .source(title)
        .children(vec![
            CaseBuilder::default()
                .test(|title: &String| {
                    if title.ends_with("1") {
                        Some(title.clone())
                    } else {
                        None
                    }
                })
                .child(|title: String| {
                    move |ctx: &mut SetupContext| {
                        ctx.create_effect_simple(move || {
                            println!("Case 1: {title}");
                        });

                        ctx.on_clean_up(|| {
                            println!("Case 1 clean up");
                        });
                    }
                })
                .build()
                .unwrap(),
            CaseBuilder::default()
                .test(|title: &String| {
                    if title.ends_with("2") {
                        Some(title.clone())
                    } else {
                        None
                    }
                })
                .child(|title: String| {
                    move |ctx: &mut SetupContext| {
                        ctx.create_effect_simple(move || {
                            println!("Case 2: {title}");
                        });

                        ctx.on_clean_up(|| {
                            println!("Case 2 clean up");
                        });
                    }
                })
                .build()
                .unwrap(),
        ])
        .build()
        .expect("To build switch")
}

pub fn content(ctx: &mut SetupContext, body: impl Signal<Value = String>) {
    ctx.create_effect(move |ctx| {
        println!("content: {}", body.get());

        ctx.add_clean_up(|| println!("content effect clean up"));
    });

    ctx.on_clean_up(|| {
        println!("content clean up");
    });
}

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();
    env_logger::init();

    let set = LocalSet::new();
    set.run_until(async move {
        let mut ctx = ReactiveContext::default();

        let root = ctx.mount_node(boxed_component(app));
        ctx.set_root(Some(root));
        ctx.poll().await;

        // select! {
        //     _ = ctx.poll() => {},
        //     _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {},
        // }
    })
    .await;
}
