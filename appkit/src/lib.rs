mod ui;

pub use ui::{Text, Window};

use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
use objc2_foundation::MainThreadMarker;
use reactive_core::{ReactiveScope, SetupContext};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, RawWaker, RawWakerVTable, Waker};

// ---------------------------------------------------------------------------
// macOS FFI — dispatch (GCD) + CoreFoundation run loop
// ---------------------------------------------------------------------------

unsafe extern "C" {
    static mut _dispatch_main_q: c_void;

    fn dispatch_async_f(
        queue: *mut c_void,
        context: *mut c_void,
        work: unsafe extern "C" fn(*mut c_void),
    );
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRunLoopRun();
    fn CFRunLoopGetMain() -> *mut c_void;
    fn CFRunLoopStop(rl: *mut c_void);
}

fn main_queue() -> *mut c_void {
    &raw mut _dispatch_main_q
}

// ---------------------------------------------------------------------------
// Shared state between waker and tick callback
// ---------------------------------------------------------------------------

struct AppState {
    scope: ReactiveScope,
    tick_scheduled: AtomicBool,
}

// ---------------------------------------------------------------------------
// Tick scheduling
// ---------------------------------------------------------------------------

fn schedule_tick(state: *mut AppState) {
    let tick_scheduled = unsafe { &(*state).tick_scheduled };
    if !tick_scheduled.swap(true, Ordering::SeqCst) {
        unsafe {
            dispatch_async_f(main_queue(), state as *mut c_void, tick_callback);
        }
    }
}

unsafe extern "C" fn tick_callback(context: *mut c_void) {
    let state = context as *mut AppState;
    let state = unsafe { &mut *state };

    state.tick_scheduled.store(false, Ordering::SeqCst);

    let waker = make_waker(state as *mut AppState);
    let mut cx = Context::from_waker(&waker);
    if state.scope.tick(&mut cx) {
        schedule_tick(state as *mut AppState);
    }
}

// ---------------------------------------------------------------------------
// Waker
// ---------------------------------------------------------------------------

fn make_waker(state: *mut AppState) -> Waker {
    unsafe fn clone(data: *const ()) -> RawWaker {
        RawWaker::new(data, &VTABLE)
    }
    unsafe fn wake(data: *const ()) {
        schedule_tick(data as *mut AppState);
    }
    unsafe fn wake_by_ref(data: *const ()) {
        schedule_tick(data as *mut AppState);
    }
    unsafe fn drop(_: *const ()) {}

    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);
    unsafe { Waker::from_raw(RawWaker::new(state as *const (), &VTABLE)) }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

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

    // Block on the run loop (services both UI events and dispatch queue)
    unsafe { CFRunLoopRun() };

    // Reclaim
    unsafe { drop(Box::from_raw(state)) };
}

/// Stop the main run loop, causing [`run_app`] to return.
pub fn stop_app() {
    unsafe { CFRunLoopStop(CFRunLoopGetMain()) };
}
