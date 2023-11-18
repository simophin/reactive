mod example;
mod waker;
mod view_component;
mod core;

use std::{
    future::Future,
    pin::Pin,
    sync::{atomic::Ordering, Arc, RwLock},
    task::Context,
};

use jni::{
    objects::{JMethodID, JObject, WeakRef},
    sys::{jboolean, jint, jlong, JNI_VERSION_1_6},
    JNIEnv, JavaVM,
};
use reactive_core::ReactiveContext;
use tokio::runtime::{Builder as RuntimeBuilder, Runtime};
use waker::WakerState;

use crate::waker::new_waker;

struct JavaReactiveContext {
    context: ReactiveContext,
    waker_state: Arc<RwLock<WakerState>>,

    tokio_rt: Runtime,

    java_instance: WeakRef,
    request_frame: JMethodID,
}

impl JavaReactiveContext {
    pub fn from(instance: jlong) -> &'static mut Self {
        unsafe { &mut *(instance as *mut JavaReactiveContext) }
    }

    pub fn poll<'local>(&mut self, env: &mut JNIEnv<'local>) -> bool {
        *self.waker_state.write().unwrap() = WakerState::LocalJavaEnv {
            wake_requested: Default::default(),
        };

        let mut poll = self.context.poll();
        let guard = self.tokio_rt.enter();
        let _ = Pin::new(&mut poll).poll(&mut Context::from_waker(&new_waker(&self.waker_state)));
        drop(guard);

        let mut state = self.waker_state.write().unwrap();

        let wake_requested = match &*state {
            WakerState::LocalJavaEnv { wake_requested } => wake_requested.load(Ordering::Relaxed),
            _ => false,
        };

        env.get_java_vm().unwrap();

        *state = WakerState::DetachedJavaEnv {
            obj: self.java_instance.clone(),
            request_wake: self.request_frame,
            vm: env.get_java_vm().expect("To get Java VM"),
        };

        wake_requested
    }
}

#[warn(non_snake_case)]
#[no_mangle]
pub extern "system" fn Java_dev_fanchao_reactive_ReactiveContext_onCreate<'local>(
    mut env: JNIEnv<'local>,
    obj: JObject<'local>,
    _state: JObject<'local>,
) -> jlong {
    let request_wake = match env
        .get_object_class(&obj)
        .and_then(|c| env.get_method_id(c, "requestFrame", "()V"))
    {
        Ok(request_wake) => request_wake,
        Err(e) => {
            let _ = env.throw_new(
                "java/lang/RuntimeException",
                format!("Failed to get requestFrame method ID: {e:?}"),
            );

            return 0;
        }
    };

    let java_instance = match env.new_weak_ref(obj) {
        Ok(Some(obj)) => obj,
        Ok(None) => {
            let _ = env.throw_new(
                "java/lang/RuntimeException",
                "Failed to create weak reference to Java object: null",
            );
            return 0;
        }

        Err(e) => {
            let _ = env.throw_new(
                "java/lang/RuntimeException",
                format!("Failed to create weak reference to Java object: {e:?}"),
            );
            return 0;
        }
    };

    let tokio_rt = match RuntimeBuilder::new_multi_thread().enable_all().build() {
        Ok(rt) => rt,
        Err(e) => {
            let _ = env.throw_new(
                "java/lang/RuntimeException",
                format!("Failed to create Tokio runtime: {e:?}"),
            );
            return 0;
        }
    };

    let context = Box::new(JavaReactiveContext {
        context: ReactiveContext::default(),
        waker_state: Default::default(),
        tokio_rt,
        request_frame: request_wake,
        java_instance,
    });

    Box::into_raw(context) as jlong
}

#[warn(non_snake_case)]
#[no_mangle]
pub extern "system" fn Java_dev_fanchao_reactive_ReactiveContext_onStart<'local>(
    _env: JNIEnv<'local>,
    _obj: JObject<'local>,
    instance: jlong,
) {
    let context = JavaReactiveContext::from(instance);
    let node = context.context.mount_node(Box::new(example::app));
    context.context.set_root(Some(node));
}

#[warn(non_snake_case)]
#[no_mangle]
pub extern "system" fn Java_dev_fanchao_reactive_ReactiveContext_onStop<'local>(
    _env: JNIEnv<'local>,
    _obj: JObject<'local>,
    instance: jlong,
) {
    let context = JavaReactiveContext::from(instance);
    context.context.set_root(None);
}

#[warn(non_snake_case)]
#[no_mangle]
pub extern "system" fn Java_dev_fanchao_reactive_ReactiveContext_onResume<'local>(
    _env: JNIEnv<'local>,
    _obj: JObject<'local>,
    _instance: jlong,
) {
    // let context = unsafe { &mut *(instance as *mut JavaReactiveContext) };
}

#[warn(non_snake_case)]
#[no_mangle]
pub extern "system" fn Java_dev_fanchao_reactive_ReactiveContext_onPause<'local>(
    _env: JNIEnv<'local>,
    _obj: JObject<'local>,
    _instance: jlong,
) {
    // let context = unsafe { &mut *(instance as *mut JavaReactiveContext) };
}

#[warn(non_snake_case)]
#[no_mangle]
pub extern "system" fn Java_dev_fanchao_reactive_ReactiveContext_onSaveInstance<'local>(
    _env: JNIEnv<'local>,
    _obj: JObject<'local>,
    _instance: jlong,
    _state: JObject<'local>,
) {
}

#[warn(non_snake_case)]
#[no_mangle]
pub extern "system" fn Java_dev_fanchao_reactive_ReactiveContext_onDestroy<'local>(
    _env: JNIEnv<'local>,
    _obj: JObject<'local>,
    instance: jlong,
) {
    let _context = unsafe { Box::from_raw(instance as *mut JavaReactiveContext) };
}

#[warn(non_snake_case)]
#[no_mangle]
pub extern "system" fn Java_dev_fanchao_reactive_ReactiveContext_handleFrame<'local>(
    mut env: JNIEnv<'local>,
    _obj: JObject<'local>,
    instance: jlong,
) -> jboolean {
    let context = unsafe { &mut *(instance as *mut JavaReactiveContext) };
    context.poll(&mut env) as jboolean
}

#[warn(non_snake_case)]
#[no_mangle]
pub extern "system" fn JNI_OnLoad(_vm: JavaVM) -> jint {
    #[cfg(target_os = "android")]
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Debug)
            .with_tag("reactive"),
    );

    JNI_VERSION_1_6
}
