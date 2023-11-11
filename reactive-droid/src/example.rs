use std::time::Duration;

use reactive_core::SetupContext;
use tokio::time::sleep;

pub fn app(ctx: &mut SetupContext) {
    ctx.create_effect_fn(move |ctx| {
        ctx.spawn(async move {
            loop {
                log::info!("Tick");
                sleep(Duration::from_secs(1)).await;
            }
        });
    });
}
