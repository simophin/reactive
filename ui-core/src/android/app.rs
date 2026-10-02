//! Activity lifecycle and the tick loop.
//!
//! Ticks are driven by the main thread's `ALooper`, the native counterpart of
//! `Handler`/`MessageQueue`. A pipe registered on the looper acts as the wake
//! signal: wakers may fire from any thread and only write one byte, so waking
//! never touches the JVM.

use super::java::{self, JavaObject};
use super::ui::ACTIVITY;
use jni::objects::JObject;
use jni::sys::{JNI_VERSION_1_6, jint, jlong};
use jni::{JNIEnv, JavaVM, NativeMethod};
use reactive_core::{ComponentId, ReactiveScope, SetupContext};
use std::ffi::{CString, c_char, c_int, c_void};
use std::io::{PipeReader, PipeWriter, Read, Write};
use std::os::fd::AsRawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::task::{Context, Wake, Waker};

#[repr(C)]
struct ALooper {
    _private: [u8; 0],
}

type ALooperCallback = unsafe extern "C" fn(fd: c_int, events: c_int, data: *mut c_void) -> c_int;

const ALOOPER_POLL_CALLBACK: c_int = -2;
const ALOOPER_EVENT_INPUT: c_int = 1;
const ANDROID_LOG_WARN: c_int = 5;
const ANDROID_LOG_ERROR: c_int = 6;

#[link(name = "android")]
unsafe extern "C" {
    fn ALooper_forThread() -> *mut ALooper;
    fn ALooper_acquire(looper: *mut ALooper);
    fn ALooper_release(looper: *mut ALooper);
    fn ALooper_addFd(
        looper: *mut ALooper,
        fd: c_int,
        ident: c_int,
        events: c_int,
        callback: Option<ALooperCallback>,
        data: *mut c_void,
    ) -> c_int;
    fn ALooper_removeFd(looper: *mut ALooper, fd: c_int) -> c_int;
}

#[link(name = "log")]
unsafe extern "C" {
    fn __android_log_write(prio: c_int, tag: *const c_char, text: *const c_char) -> c_int;
}

static SETUP: OnceLock<fn(&mut SetupContext)> = OnceLock::new();

/// Wake signal shared with every [`Waker`] handed out by the tick loop.
///
/// Both pipe ends live here so a waker outliving the activity writes into an
/// unread pipe instead of a closed one (which would raise `SIGPIPE`).
struct TickSignal {
    scheduled: AtomicBool,
    reader: PipeReader,
    writer: PipeWriter,
}

impl Wake for TickSignal {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        // At most one byte is ever in flight: it is written on the
        // false -> true transition and consumed by `on_tick`.
        if !self.scheduled.swap(true, Ordering::AcqRel) {
            let _ = (&self.writer).write_all(&[1]);
        }
    }
}

struct AppState {
    scope: ReactiveScope,
    root: ComponentId,
    looper: *mut ALooper,
    signal: Arc<TickSignal>,
    waker: Waker,
}

/// Entry point for [`android_main!`](crate::android_main). Stores the app's
/// setup function and registers the native methods of the `com.reactive`
/// Java classes.
///
/// # Safety
/// `vm` must be the `JavaVM` pointer passed to `JNI_OnLoad`.
pub unsafe fn on_load(vm: *mut jni::sys::JavaVM, setup: fn(&mut SetupContext)) -> jint {
    install_panic_hook();
    let vm = unsafe { JavaVM::from_raw(vm) }.expect("valid JavaVM");
    let mut env = vm.get_env().expect("JNI_OnLoad runs on an attached thread");

    register(
        &mut env,
        "com/reactive/ReactiveActivity",
        &[
            ("nativeCreate", "()J", native_create as *mut c_void),
            ("nativeDestroy", "(J)V", native_destroy as *mut c_void),
        ],
    );
    super::ui::flex::register_natives(&mut env);
    super::callback::register_natives(&mut env);

    java::init_vm(vm);
    let _ = SETUP.set(setup);
    JNI_VERSION_1_6
}

pub(crate) fn register(env: &mut JNIEnv, class: &str, methods: &[(&str, &str, *mut c_void)]) {
    let methods: Vec<_> = methods
        .iter()
        .map(|&(name, sig, fn_ptr)| NativeMethod {
            name: name.into(),
            sig: sig.into(),
            fn_ptr,
        })
        .collect();
    let result = env.register_native_methods(class, &methods);
    java::check(env, result, class);
}

extern "system" fn native_create(env: JNIEnv, activity: JObject) -> jlong {
    let setup = SETUP.get().expect("android_main! setup function");
    let activity = JavaObject::new(&env, &activity);

    let (reader, writer) = std::io::pipe().expect("create tick pipe");
    let signal = Arc::new(TickSignal {
        scheduled: AtomicBool::new(false),
        reader,
        writer,
    });

    let scope = ReactiveScope::default();
    let mut ctx = SetupContext::new_root(&scope);
    ctx.set_static_context(&ACTIVITY, activity);
    setup(&mut ctx);

    let looper = unsafe { ALooper_forThread() };
    assert!(
        !looper.is_null(),
        "nativeCreate must run on a looper thread"
    );
    unsafe { ALooper_acquire(looper) };

    let fd = signal.reader.as_raw_fd();
    let waker = Waker::from(signal.clone());
    waker.wake_by_ref();

    let state = Box::into_raw(Box::new(AppState {
        scope,
        root: ctx.component_id(),
        looper,
        waker,
        signal,
    }));

    unsafe {
        ALooper_addFd(
            looper,
            fd,
            ALOOPER_POLL_CALLBACK,
            ALOOPER_EVENT_INPUT,
            Some(on_tick),
            state.cast(),
        );
    }

    state as jlong
}

extern "system" fn native_destroy(_env: JNIEnv, _activity: JObject, handle: jlong) {
    let state = unsafe { Box::from_raw(handle as *mut AppState) };
    unsafe {
        ALooper_removeFd(state.looper, state.signal.reader.as_raw_fd());
        ALooper_release(state.looper);
    }
    state.scope.dispose_component(state.root);
}

unsafe extern "C" fn on_tick(_fd: c_int, _events: c_int, data: *mut c_void) -> c_int {
    let state = unsafe { &mut *(data as *mut AppState) };

    let mut byte = [0u8; 1];
    let _ = (&state.signal.reader).read_exact(&mut byte);
    state.signal.scheduled.store(false, Ordering::Release);

    // This callback is not a JNI call, so local references created during the
    // tick would never be freed without an explicit frame.
    let mut env = java::env();
    let _ = env.with_local_frame(64, |_| -> jni::errors::Result<()> {
        state.scope.tick(&mut Context::from_waker(&state.waker));
        Ok(())
    });

    1
}

fn log(priority: c_int, message: &str) {
    if let Ok(text) = CString::new(message) {
        unsafe { __android_log_write(priority, c"reactive".as_ptr(), text.as_ptr()) };
    }
}

pub(crate) fn log_warn(message: &str) {
    log(ANDROID_LOG_WARN, message);
}

/// Panics would otherwise only reach stderr, which Android discards.
fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log(ANDROID_LOG_ERROR, &info.to_string());
        default_hook(info);
    }));
}
