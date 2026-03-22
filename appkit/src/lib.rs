mod ui;

pub use ui::*;

use apple::app_loop::{AppState, schedule_tick};
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
use objc2_foundation::MainThreadMarker;
use reactive_core::{ReactiveScope, SetupContext};
use std::sync::atomic::AtomicBool;

/// Start the app. Initializes NSApplication, runs `setup` to build the
/// component tree, then enters the macOS main run loop.
/// This function blocks until [`stop_app`] is called.
pub fn run_app(setup: impl FnOnce(&mut SetupContext)) {
    let mtm = MainThreadMarker::new().expect("run_app must be called on the main thread");

    // Initialize NSApplication for UI
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Regular);

    // Set up reactive scope
    let mut scope = ReactiveScope::default();
    setup(&mut SetupContext::new_root(&mut scope));

    let state = Box::into_raw(Box::new(AppState {
        scope,
        tick_scheduled: AtomicBool::new(false),
    }));

    schedule_tick(state);

    // Bring app to front
    app.activate();

    // Block until the app quits — NSApplication's event loop services
    // both AppKit UI events and the GCD main queue (where ticks are posted).
    app.run();

    // Reclaim
    unsafe { drop(Box::from_raw(state)) };
}

/// Stop the app, causing [`run_app`] to return.
pub fn stop_app() {
    let mtm = unsafe { MainThreadMarker::new_unchecked() };
    NSApplication::sharedApplication(mtm).terminate(None);
}
