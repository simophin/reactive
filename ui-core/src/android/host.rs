use super::logcat;
use super::node::{self, Node, check};
use super::ui::RootRegistry;
use crate::widgets::NativeViewRegistry;
use jni::JNIEnv;
use jni::objects::{GlobalRef, JClass, JObject};
use jni::sys::jlong;
use reactive_core::{ComponentId, ReactiveScope, SetupContext};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Wake, Waker};

/// Wakes the reactive scope by asking `ReactiveHost` to post a tick to the main looper.
/// It only touches the host's global reference, so it can be woken from any thread.
struct TickScheduler {
    scheduled: AtomicBool,
    host: GlobalRef,
}

impl Wake for TickScheduler {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        if !self.scheduled.swap(true, Ordering::SeqCst) {
            let mut env = node::env();
            let result = env.call_method(self.host.as_obj(), "scheduleTick", "()V", &[]);
            check(&mut env, result, "scheduleTick");
        }
    }
}

struct HostState {
    scope: ReactiveScope,
    root: ComponentId,
    waker: Waker,
    scheduler: Arc<TickScheduler>,
}

/// Implementation of [`android_main!`](crate::android_main): creates the scope for a
/// `com.reactive.ReactiveHost`, builds the component tree into its root node, and returns the
/// pointer Kotlin passes back to `nativeTick` / `nativeDestroy`.
pub fn create_host(env: JNIEnv, host: JObject, setup: impl FnOnce(&mut SetupContext)) -> jlong {
    let mut env = env;
    logcat::install_panic_hook();
    node::init_java_vm(&env);

    let root = env.call_method(&host, "getRoot", "()Lcom/reactive/RootNode;", &[]);
    let root = check(&mut env, root.and_then(|v| v.l()), "getRoot");
    let root = Node::from_local(&mut env, root);

    let host = env.new_global_ref(host);
    let scheduler = Arc::new(TickScheduler {
        scheduled: AtomicBool::new(false),
        host: check(&mut env, host, "new_global_ref"),
    });

    let scope = ReactiveScope::default();
    let mut ctx = SetupContext::new_root(&scope);
    let registry: Rc<dyn NativeViewRegistry<Node>> = Rc::new(RootRegistry {
        root,
        current: Default::default(),
    });
    ctx.set_static_context(&super::ui::VIEW_REGISTRY_KEY, registry);
    setup(&mut ctx);

    let state = Box::new(HostState {
        root: ctx.component_id(),
        scope,
        waker: Waker::from(scheduler.clone()),
        scheduler,
    });

    // The first tick installs the waker that later signal writes use.
    state.waker.wake_by_ref();
    Box::into_raw(state) as jlong
}

#[unsafe(no_mangle)]
extern "system" fn Java_com_reactive_ReactiveHost_nativeTick(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) {
    // SAFETY: `ptr` came from `create_host` and Kotlin stops ticking before destroying it.
    let state = unsafe { &*(ptr as *const HostState) };
    state.scheduler.scheduled.store(false, Ordering::SeqCst);
    state.scope.tick(&mut Context::from_waker(&state.waker));
}

#[unsafe(no_mangle)]
extern "system" fn Java_com_reactive_ReactiveHost_nativeDestroy(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) {
    // SAFETY: `ptr` came from `create_host`; Kotlin zeroes its copy after this call.
    let state = unsafe { Box::from_raw(ptr as *mut HostState) };
    // Run cleanups so callbacks and layout trees are released.
    state.scope.dispose_component(state.root);
}
