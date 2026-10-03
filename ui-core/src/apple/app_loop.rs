//! Tick loop on the GCD main queue, shared by AppKit and UIKit.
//!
//! Wakers may fire from any thread (for example tokio workers): waking only
//! flips an atomic flag and posts one `dispatch_async_f` to the main queue.
//! The reactive scope itself is only ever touched on the main thread.

use objc2::MainThreadMarker;
use reactive_core::ReactiveScope;
use std::cell::RefCell;
use std::ffi::c_void;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Wake, Waker};

unsafe extern "C" {
    static mut _dispatch_main_q: c_void;

    fn dispatch_async_f(
        queue: *mut c_void,
        context: *mut c_void,
        work: unsafe extern "C" fn(*mut c_void),
    );
}

fn main_queue() -> *mut c_void {
    &raw mut _dispatch_main_q
}

struct TickState {
    scheduled: AtomicBool,
    /// Only read or written on the main thread.
    scope: RefCell<Option<ReactiveScope>>,
}

// SAFETY: `scope` is only accessed on the main thread (by `AppLoop`, which is
// `!Send`, and by `tick_callback`, which runs on the main queue). Other
// threads only touch the atomic flag. `Drop` never drops the scope off the
// main thread.
unsafe impl Send for TickState {}
unsafe impl Sync for TickState {}

impl Wake for TickState {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        if !self.scheduled.swap(true, Ordering::AcqRel) {
            let context = Arc::into_raw(self.clone()) as *mut c_void;
            unsafe { dispatch_async_f(main_queue(), context, tick_callback) };
        }
    }
}

impl Drop for TickState {
    fn drop(&mut self) {
        if let Some(scope) = self.scope.get_mut().take() {
            if MainThreadMarker::new().is_some() {
                drop(scope);
            } else {
                // The scope is `Rc`-based and must not be dropped here.
                std::mem::forget(scope);
            }
        }
    }
}

unsafe extern "C" fn tick_callback(context: *mut c_void) {
    // Balances the `Arc::into_raw` in `wake_by_ref`.
    let state = unsafe { Arc::from_raw(context as *const TickState) };
    state.scheduled.store(false, Ordering::Release);

    // Clone out of the cell so no borrow is held while effects run.
    let scope = state.scope.borrow().clone();
    if let Some(scope) = scope {
        let waker = Waker::from(state.clone());
        scope.tick(&mut Context::from_waker(&waker));
    }
}

/// Drives a [`ReactiveScope`] from the GCD main queue.
///
/// Dropping the loop (or calling [`AppLoop::shutdown`]) detaches the scope;
/// wakers that are still alive then become no-ops.
pub(crate) struct AppLoop {
    state: Arc<TickState>,
    _main_thread: MainThreadMarker,
}

impl AppLoop {
    pub fn new(scope: ReactiveScope, mtm: MainThreadMarker) -> Self {
        Self {
            state: Arc::new(TickState {
                scheduled: AtomicBool::new(false),
                scope: RefCell::new(Some(scope)),
            }),
            _main_thread: mtm,
        }
    }

    /// Schedules a tick on the main queue unless one is already pending.
    pub fn schedule_tick(&self) {
        self.state.wake_by_ref();
    }

    /// Detaches and returns the scope. Pending and future ticks do nothing.
    pub fn shutdown(&self) -> Option<ReactiveScope> {
        self.state.scope.borrow_mut().take()
    }
}

impl Drop for AppLoop {
    fn drop(&mut self) {
        drop(self.shutdown());
    }
}
