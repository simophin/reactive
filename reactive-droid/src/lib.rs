mod waker;

use std::task::{RawWakerVTable, Waker};

use jni::{
    objects::{GlobalRef, JObject},
    sys::jlong,
    JNIEnv, JavaVM,
};
use reactive_core::ReactiveContext;

struct JavaReactiveContext {
    context: ReactiveContext,
    java_instance: GlobalRef,
}


#[warn(non_snake_case)]
#[no_mangle]
pub extern "system" fn Java_dev_fanchao_reactive_ReactiveContext_onCreate<'local>(
    env: JNIEnv<'local>,
    obj: JObject<'local>,
    state: JObject<'local>,
) -> jlong {
    let context = Box::new(JavaReactiveContext {
        context: ReactiveContext::default(),
        java_instance: env.new_global_ref(obj).unwrap(),
    });

    Box::into_raw(context) as jlong
}

#[warn(non_snake_case)]
#[no_mangle]
pub extern "system" fn Java_dev_fanchao_reactive_ReactiveContext_onStart<'local>(
    env: JNIEnv<'local>,
    obj: JObject<'local>,
    instance: jlong,
) {
    let context = unsafe { &mut *(instance as *mut JavaReactiveContext) };
    let mut poll = context.context.poll();
}
#[warn(non_snake_case)]
#[no_mangle]
pub extern "system" fn Java_dev_fanchao_reactive_ReactiveContext_onStop<'local>(
    env: JNIEnv<'local>,
    obj: JObject<'local>,
    instance: jlong,
) {
    // let context = unsafe { &mut *(instance as *mut JavaReactiveContext) };
}

#[warn(non_snake_case)]
#[no_mangle]
pub extern "system" fn Java_dev_fanchao_reactive_ReactiveContext_onResume<'local>(
    env: JNIEnv<'local>,
    obj: JObject<'local>,
    instance: jlong,
) {
    // let context = unsafe { &mut *(instance as *mut JavaReactiveContext) };
}

#[warn(non_snake_case)]
#[no_mangle]
pub extern "system" fn Java_dev_fanchao_reactive_ReactiveContext_onPause<'local>(
    env: JNIEnv<'local>,
    obj: JObject<'local>,
    instance: jlong,
) {
    // let context = unsafe { &mut *(instance as *mut JavaReactiveContext) };
}

#[warn(non_snake_case)]
#[no_mangle]
pub extern "system" fn Java_dev_fanchao_reactive_ReactiveContext_onDestroy<'local>(
    env: JNIEnv<'local>,
    obj: JObject<'local>,
    instance: jlong,
) {
    let context = unsafe { Box::from_raw(instance as *mut JavaReactiveContext) };
}
