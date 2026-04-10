pub mod app_loop;
pub mod bindings;
pub mod desc;
pub mod ui;

use jni::objects::{JClass, JObject};
use jni::sys::jlong;
use jni::JNIEnv;
use reactive_core::ReactiveScope;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::Context;

use crate::app_loop::AppState;

#[no_mangle]
pub extern "C" fn Java_com_reactive_ReactiveScope_nativeCreate(
    env: JNIEnv,
    _class: JClass,
) -> jlong {
    let scope = ReactiveScope::default();
    let tick_scheduled = Arc::new(AtomicBool::new(false));
    crate::ui::view_component::AndroidView::set_java_vm(
        env.get_java_vm().expect("load JavaVM for view runtime"),
    );
    let state = Arc::new(AppState {
        scope,
        tick_scheduled: tick_scheduled.clone(),
        java_vm: env.get_java_vm().expect("load JavaVM"),
    });
    Arc::into_raw(state) as jlong
}

#[no_mangle]
pub extern "C" fn Java_com_reactive_ReactiveScope_nativeDestroy(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) {
    if ptr != 0 {
        crate::ui::view_component::AndroidView::clear_activity();
        unsafe { drop(Arc::from_raw(ptr as *const AppState)) };
    }
}

#[no_mangle]
pub extern "C" fn Java_com_reactive_ReactiveScope_nativeAttachActivity(
    mut env: JNIEnv,
    _class: JClass,
    _ptr: jlong,
    activity: JObject,
) {
    crate::ui::view_component::AndroidView::set_activity(&mut env, activity);
}

#[no_mangle]
pub extern "C" fn Java_com_reactive_ReactiveScope_nativeTick(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) {
    unsafe { Arc::increment_strong_count(ptr as *const AppState) };
    let state = unsafe { Arc::from_raw(ptr as *const AppState) };
    state.tick_scheduled.store(false, Ordering::SeqCst);
    let waker = crate::app_loop::make_android_waker(state.clone());
    let mut ctx = Context::from_waker(&waker);
    state.scope.tick(&mut ctx);
}
