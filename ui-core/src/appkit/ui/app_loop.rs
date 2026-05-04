use reactive_core::ReactiveScope;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, RawWaker, RawWakerVTable, Waker};

// ---------------------------------------------------------------------------
// macOS/iOS FFI — dispatch (GCD)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    static mut _dispatch_main_q: c_void;

    fn dispatch_async_f(
        queue: *mut c_void,
        context: *mut c_void,
        work: unsafe extern "C" fn(*mut c_void),
    );
}

unsafe fn main_queue() -> *mut c_void {
    &raw mut _dispatch_main_q
}

// ---------------------------------------------------------------------------
// Shared app state
// ---------------------------------------------------------------------------

pub struct AppState {
    pub scope: ReactiveScope,
    pub tick_scheduled: AtomicBool,
}

// ---------------------------------------------------------------------------
// Tick scheduling
// ---------------------------------------------------------------------------

pub fn schedule_tick(state: *mut AppState) {
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
    state.scope.tick(&mut cx);
}

// ---------------------------------------------------------------------------
// Waker
// ---------------------------------------------------------------------------

pub fn make_waker(state: *mut AppState) -> Waker {
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
