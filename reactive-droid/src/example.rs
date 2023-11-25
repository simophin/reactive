use std::time::Duration;

use crate::core::text::*;
use crate::core::view::*;
use jni::objects::GlobalRef;
use reactive_core::SignalGetter;
use reactive_core::{core_component::*, Component, SetupContext, SingleValue};
use reactive_derive::{component, jsx};
use tokio::time::sleep;

#[component]
pub fn app(ctx: &mut SetupContext, activity: GlobalRef) -> impl Component {
    let (counter, set_counter) = ctx.create_signal(0);
    ctx.create_effect_fn(move |ctx| {
        let mut set_counter = set_counter.clone();
        ctx.spawn(async move {
            loop {
                log::info!("Tick");
                sleep(Duration::from_secs(1)).await;
                set_counter.update_with(|v| {
                    *v += 1;
                    true
                });
            }
        });
    });

    let text = move || {
        format!(
            "Hello, world! You have clicked the button {} times.",
            counter.get()
        )
    };

    jsx! {
        <Provider key=&ANDROID_VIEW_CONTAINER_KEY value=SingleValue(AndroidViewContainer::Activity(activity))>
            <TextView text=text  />
        </Provider>
    }
}
