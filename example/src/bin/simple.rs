use std::time::Duration;

use reactive_core::{
    boxed_component,
    core_component::{CaseBuilder, ShowBuilder, SwitchBuilder},
    Component, LoadState, ReactiveContext, ResourceResult, SetupContext, Signal, SignalGetter,
};
use reactive_macros::jsx;
use tokio::{task::LocalSet, time::sleep};

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

    ctx.create_effect_fn(move |ctx| {
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

    jsx! {
            <Show test=move || { index.get() > 2 } >
            {move || {
                let body = body.clone();
                move |ctx: &mut SetupContext| content(ctx, body.clone())
            }}
            </Show>
    }

    // let show = ShowBuilder::default()
    //     .test(move || index.get() % 2 == 0)
    //     .success(move || {
    //         let body = body.clone();
    //         move |ctx: &mut SetupContext| content(ctx, body.clone())
    //     })
    //     .fail(|| ())
    //     .build()
    //     .unwrap();

    // vec![
    //     boxed_component(move |ctx: &mut SetupContext| header(ctx, title.clone())),
    //     boxed_component(show),
    // ]
}

pub fn header(ctx: &mut SetupContext, title: impl Signal<Value = String>) -> impl Component {
    ctx.on_clean_up(|| {
        println!("header clean up");
    });

    return jsx! {
        <Switch source=title fallback=move || ()>
            <Case test=|title: &String| {
                if title.ends_with("1") {
                    Some(title.clone())
                } else {
                    None
                }
            }>
            {|title: String| {
                move |ctx: &mut SetupContext| {
                    ctx.create_effect_simple(move || {
                        println!("Case 1: {title}");
                    });

                    ctx.on_clean_up(|| {
                        println!("Case 1 clean up");
                    });
                }
            }}
            </Case>

            <Case test=|title: &String| {
                if title.ends_with("2") {
                    Some(title.clone())
                } else {
                    None
                }
            }>
            {|title: String| {
                move |ctx: &mut SetupContext| {
                    ctx.create_effect_simple(move || {
                        println!("Case 2: {title}");
                    });

                    ctx.on_clean_up(|| {
                        println!("Case 2 clean up");
                    });
                }
            }}
            </Case>
        </Switch>
    };
}

pub fn content(ctx: &mut SetupContext, body: impl Signal<Value = String>) {
    let ResourceResult {
        mut trigger,
        state,
        update,
    } = ctx.create_resource((), |_| async move {
        sleep(Duration::from_secs(10)).await;
        "Future result"
    });

    ctx.create_effect_simple(move || {
        println!("Future load result: {:?}", state.get());

        if state.with(|v| v.state) == LoadState::Loaded {
            println!("Reload result");
            trigger();
        }
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
