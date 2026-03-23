use jni::JNIEnv;
use jni::objects::JClass;
use jni::sys::jlong;
use reactive_core::ReactiveScope;
use std::task::{Context, Waker};

#[no_mangle]
pub extern "C" fn Java_com_reactive_ReactiveScope_nativeCreate(
    _env: JNIEnv,
    _class: JClass,
) -> jlong {
    Box::into_raw(Box::new(ReactiveScope::default())) as jlong
}

#[no_mangle]
pub extern "C" fn Java_com_reactive_ReactiveScope_nativeDestroy(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) {
    if ptr != 0 {
        unsafe { drop(Box::from_raw(ptr as *mut ReactiveScope)) };
    }
}

#[no_mangle]
pub extern "C" fn Java_com_reactive_ReactiveScope_nativeTick(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) {
    let scope = unsafe { &mut *(ptr as *mut ReactiveScope) };
    let waker = Waker::noop();
    let mut ctx = Context::from_waker(&waker);
    scope.tick(&mut ctx);
}
