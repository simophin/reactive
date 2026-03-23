use reactive_core::{SetupContext, StoredSignal};

/// Shared application state — created once at startup and handed to each
/// platform's entry point.  Platform code wires native views to these signals.
pub struct AppState {
    pub count: StoredSignal<i32>,
}

/// Set up the reactive component tree.  Called by every platform entry point
/// with the root `SetupContext`.
pub fn setup(ctx: &mut SetupContext) -> AppState {
    AppState {
        count: ctx.create_signal(0i32),
    }
}
