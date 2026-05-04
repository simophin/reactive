mod ui;
pub use ui::*;

use gtk4::glib;
use gtk4::prelude::*;
use reactive_core::{ReactiveScope, SetupContext};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, RawWaker, RawWakerVTable, Waker};

struct AppState {
    scope: ReactiveScope,
    tick_scheduled: AtomicBool,
}

fn schedule_tick(state: *mut AppState) {
    let tick_scheduled = unsafe { &(*state).tick_scheduled };
    if !tick_scheduled.swap(true, Ordering::SeqCst) {
        let ptr = state as usize;
        glib::idle_add_once(move || tick_callback(ptr as *mut AppState));
    }
}

fn tick_callback(state: *mut AppState) {
    let state = unsafe { &mut *state };
    state.tick_scheduled.store(false, Ordering::SeqCst);
    let waker = make_waker(state as *mut AppState);
    let mut cx = Context::from_waker(&waker);
    state.scope.tick(&mut cx);
}

fn make_waker(state: *mut AppState) -> Waker {
    let ptr = state as *const ();
    unsafe { Waker::from_raw(RawWaker::new(ptr, &WAKER_VTABLE)) }
}

static WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
    |ptr| RawWaker::new(ptr, &WAKER_VTABLE),
    |ptr| schedule_tick(ptr as *mut AppState),
    |ptr| schedule_tick(ptr as *mut AppState),
    |_| {},
);

/// Start the GTK application. Initialises GTK, runs `setup` to build the
/// component tree inside the `activate` signal, then enters the GLib main loop.
/// Blocks until [`stop_app`] is called.
pub fn run_app(setup: impl FnOnce(&mut SetupContext) + 'static) {
    let app = gtk4::Application::builder()
        .application_id("com.reactive.demo")
        .build();

    let setup = RefCell::new(Some(Box::new(setup) as Box<dyn FnOnce(&mut SetupContext)>));
    let state_ptr: Rc<Cell<usize>> = Rc::new(Cell::new(0));
    let state_ptr2 = Rc::clone(&state_ptr);

    app.connect_activate(move |_app| {
        if let Some(setup_fn) = setup.borrow_mut().take() {
            let scope = ReactiveScope::default();
            setup_fn(&mut SetupContext::new_root(&scope));
            let state = Box::into_raw(Box::new(AppState {
                scope,
                tick_scheduled: AtomicBool::new(false),
            }));
            state_ptr2.set(state as usize);
            schedule_tick(state);
        }
    });

    app.run();

    let ptr = state_ptr.get();
    if ptr != 0 {
        unsafe { drop(Box::from_raw(ptr as *mut AppState)) };
    }
}

/// Stop the app, causing [`run_app`] to return.
pub fn stop_app() {
    use gtk4::gio::prelude::ApplicationExt;
    if let Some(app) = gtk4::gio::Application::default() {
        app.quit();
    }
}
