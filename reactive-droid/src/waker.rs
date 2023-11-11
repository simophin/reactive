use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock, Weak,
    },
    task::{RawWaker, RawWakerVTable, Waker},
};

use jni::{
    objects::{JMethodID, WeakRef},
    signature::{Primitive, ReturnType},
    JavaVM,
};

pub enum WakerState {
    LocalJavaEnv {
        wake_requested: AtomicBool,
    },

    DetachedJavaEnv {
        vm: JavaVM,
        obj: WeakRef,
        request_wake: JMethodID,
    },
}

impl Default for WakerState {
    fn default() -> Self {
        WakerState::LocalJavaEnv {
            wake_requested: AtomicBool::new(false),
        }
    }
}

impl WakerState {
    pub fn wake(this: &Weak<RwLock<Self>>) {
        let Some(this) = this.upgrade() else {
            return;
        };

        let this = this.read().unwrap();
        match &*this {
            WakerState::LocalJavaEnv { wake_requested } => {
                wake_requested.store(true, Ordering::Relaxed);
                return;
            }

            WakerState::DetachedJavaEnv {
                vm,
                obj,
                request_wake,
            } => {
                let Ok(mut guard) = vm.attach_current_thread() else {
                    return;
                };

                let Ok(Some(obj)) = obj.upgrade_local(&guard) else {
                    return;
                };

                let _ = unsafe {
                    guard.call_method_unchecked(
                        obj,
                        request_wake,
                        ReturnType::Primitive(Primitive::Void),
                        &[],
                    )
                };

                let _ = guard.exception_check();
            }
        }
    }
}

fn waker_from(data: *const ()) -> Weak<RwLock<WakerState>> {
    unsafe { Weak::from_raw(data as *const RwLock<WakerState>) }
}

fn waker_clone(data: *const ()) -> RawWaker {
    let data = waker_from(data);
    let waker = data.clone();
    let _ = Weak::into_raw(data);
    RawWaker::new(waker.into_raw() as *const (), &WAKER_VTABLE)
}

fn waker_wake(data: *const ()) {
    let data = waker_from(data);
    WakerState::wake(&data);
}

fn waker_wake_by_ref(data: *const ()) {
    let data = waker_from(data);
    WakerState::wake(&data);
    let _ = Weak::into_raw(data);
}

fn waker_drop(data: *const ()) {
    let _ = waker_from(data);
}

const WAKER_VTABLE: RawWakerVTable =
    RawWakerVTable::new(waker_clone, waker_wake, waker_wake_by_ref, waker_drop);

pub fn new_waker(state: &Arc<RwLock<WakerState>>) -> Waker {
    let state = Arc::downgrade(state);
    unsafe { Waker::from_raw(RawWaker::new(state.into_raw() as *const (), &WAKER_VTABLE)) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weak_ref() {
        let t = Arc::new(1);

        let weak = Arc::downgrade(&t);
        let orig_weak_ptr = weak.into_raw();
        let weak = unsafe { Weak::from_raw(orig_weak_ptr) };
        assert_eq!(orig_weak_ptr, weak.into_raw());
    }
}
