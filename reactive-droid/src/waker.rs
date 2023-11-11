use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock, Weak,
    },
    task::{RawWaker, RawWakerVTable, Waker},
};

use jni::{
    objects::{JMethodID, JStaticMethodID, WeakRef},
    signature::ReturnType,
    sys::{jobject, JNIEnv},
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

fn waker_clone(data: *const ()) -> RawWaker {
    let data = unsafe { Weak::from_raw(data as *const RwLock<WakerState>) };
    let waker = data.clone();
    let _ = Weak::into_raw(data);
    RawWaker::new(waker.into_raw() as *const (), &waker_vtable)
}

fn waker_wake(data: *const ()) {
    let data = unsafe { Weak::from_raw(data as *const RwLock<WakerState>) };
    let Some(data) = data.upgrade() else {
        return;
    };

    let data = data.read().unwrap();
    match &*data {
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

            let _ =
                unsafe { guard.call_method_unchecked(obj, request_wake, ReturnType::Object, &[]) };
        }
    }
}

fn waker_wake_by_ref(data: *const ()) {}

fn waker_drop(data: *const ()) {}

const waker_vtable: RawWakerVTable =
    RawWakerVTable::new(waker_clone, waker_wake, waker_wake_by_ref, waker_drop);

pub fn new_waker(state: &Arc<RwLock<WakerState>>) -> Waker {
    let state = Arc::downgrade(state);
    unsafe { Waker::from_raw(RawWaker::new(state.into_raw() as *const (), &waker_vtable)) }
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
