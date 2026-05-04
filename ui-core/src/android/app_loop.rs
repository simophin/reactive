use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{RawWaker, RawWakerVTable, Waker};

use jni::JavaVM;
use reactive_core::ReactiveScope;

pub struct AppState {
    pub scope: ReactiveScope,
    pub tick_scheduled: Arc<AtomicBool>,
    pub java_vm: JavaVM,
}

fn clone_ptr(ptr: *const ()) -> RawWaker {
    unsafe { Arc::increment_strong_count(ptr as *const AppState) };
    RawWaker::new(ptr, &VTABLE)
}

fn wake_ptr(ptr: *const ()) {
    let state = unsafe { Arc::from_raw(ptr as *const AppState) };
    state.tick_scheduled.store(true, Ordering::SeqCst);
    schedule_tick(&state.java_vm, &state.tick_scheduled);
    std::mem::forget(state);
}

fn drop_ptr(ptr: *const ()) {
    unsafe { Arc::decrement_strong_count(ptr as *const AppState) };
}

static VTABLE: RawWakerVTable = RawWakerVTable::new(clone_ptr, wake_ptr, wake_ptr, drop_ptr);

pub fn make_android_waker(state: Arc<AppState>) -> Waker {
    let ptr = Arc::into_raw(state) as *const ();
    unsafe { Waker::from_raw(RawWaker::new(ptr, &VTABLE)) }
}

pub fn schedule_tick(java_vm: &JavaVM, tick_scheduled: &Arc<AtomicBool>) {
    if !tick_scheduled.swap(true, Ordering::SeqCst) {
        // Post a Runnable to the Android main Handler that calls nativeTick.
        // The Kotlin side ReactiveScope.scheduleTick() does this:
        //   Handler(Looper.getMainLooper()).post { nativeTick() }
        if let Ok(mut env) = java_vm.get_env() {
            let _ =
                env.call_static_method("com/reactive/ReactiveScope", "scheduleTick", "()V", &[]);
        }
    }
}
