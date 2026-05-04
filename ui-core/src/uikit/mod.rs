mod ui;

pub use ui::*;

use apple::app_loop::{AppState, make_waker, schedule_tick};
use objc2_foundation::MainThreadMarker;
use objc2_ui_kit::UIApplication;
use reactive_core::{ReactiveScope, SetupContext};
use std::sync::atomic::AtomicBool;

/// Start the app. Initializes the reactive scope, runs `setup` to build the
/// component tree, then schedules the first tick.
///
/// This should be called from within your `UIApplicationDelegate`
/// `application(_:didFinishLaunchingWithOptions:)` or equivalent entry point.
pub fn run_setup(setup: impl FnOnce(&mut SetupContext)) -> *mut AppState {
    let mut scope = ReactiveScope::default();
    setup(&mut SetupContext::new_root(&mut scope));

    let state = Box::into_raw(Box::new(AppState {
        scope,
        tick_scheduled: AtomicBool::new(false),
    }));

    schedule_tick(state);
    state
}

/// Stop the app, causing the UIApplication to terminate.
pub fn stop_app() {
    let mtm = unsafe { MainThreadMarker::new_unchecked() };
    UIApplication::sharedApplication(mtm).terminate(None);
}
