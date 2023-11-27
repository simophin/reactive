use crate::core::text::*;
use crate::core::view::*;
use jni::objects::GlobalRef;
use reactive_core::SignalGetter;
use reactive_core::{core_component::*, SetupContext, SingleValue};
use reactive_derive::{component, jsx};

#[component]
pub fn app(ctx: &mut SetupContext, activity: GlobalRef) {
    let (counter, mut set_counter) = ctx.create_signal(0);
    // ctx.create_effect_fn(move |ctx| {
    //     let mut set_counter = set_counter.clone();
    //     ctx.spawn(async move {
    //         loop {
    //             log::info!("Tick");
    //             sleep(Duration::from_secs(1)).await;
    //             set_counter.update_with(|v| {
    //                 *v += 1;
    //                 true
    //             });
    //         }
    //     });
    // });

    let text = move || {
        format!(
            "Hello, world! You have clicked the button {} times.",
            counter.get()
        )
    };

    ctx.children.push(Box::new(
        jsx! {
            <Provider key=&ANDROID_VIEW_CONTAINER_KEY value=SingleValue(AndroidViewContainer::Activity(activity))>
                <TextView text=text on_click=move || {
                    log::info!("Clicked");
                    set_counter.update_with(|v| {
                        *v += 1;
                        true
                    });
                }  />
            </Provider>
        }
    ));
}
