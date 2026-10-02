use super::node::Node;
use jni::JNIEnv;
use jni::objects::{JObject, JValue};
use jni::sys::jlong;
use reactive_core::SetupContext;
use std::cell::RefCell;

type Handler = RefCell<Box<dyn FnMut(&mut JNIEnv, i64, JObject) -> i64>>;

/// A Rust closure exposed to Kotlin as a `com.reactive.NativeCallback`.
///
/// Kotlin invokes it as `invoke(value: Long, arg: Any?): Long`; what `value` and `arg` mean
/// is up to the node that holds the callback. Dropping the `Callback` releases the Kotlin
/// object, so events arriving afterwards are ignored.
pub struct Callback {
    object: Node,
    handler: *mut Handler,
}

impl Callback {
    pub fn new(handler: impl FnMut(&mut JNIEnv, i64, JObject) -> i64 + 'static) -> Self {
        let handler: *mut Handler = Box::into_raw(Box::new(RefCell::new(Box::new(handler))));
        let object = Node::new_with(
            "com/reactive/NativeCallback",
            "(J)V",
            &[JValue::Long(handler as jlong)],
        );
        Self { object, handler }
    }

    /// Creates a callback, hands it to `node` through `setter` (a Kotlin
    /// `var x: NativeCallback?`), and releases it when the component is disposed.
    pub fn attach(
        ctx: &SetupContext,
        node: &Node,
        setter: &str,
        handler: impl FnMut(&mut JNIEnv, i64, JObject) -> i64 + 'static,
    ) {
        let callback = Self::new(handler);
        node.call(
            setter,
            "(Lcom/reactive/NativeCallback;)V",
            &[JValue::Object(callback.object.as_obj())],
        );
        ctx.on_cleanup(move || drop(callback));
    }
}

impl Drop for Callback {
    fn drop(&mut self) {
        self.object.call("release", "()V", &[]);
        drop(unsafe { Box::from_raw(self.handler) });
    }
}

#[unsafe(no_mangle)]
extern "system" fn Java_com_reactive_NativeCallback_nativeInvoke<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    handler: jlong,
    value: jlong,
    arg: JObject<'local>,
) -> jlong {
    // SAFETY: Kotlin only holds a non-zero pointer until `Callback::drop` releases it, and
    // both run on the main thread.
    let handler = unsafe { &*(handler as *const Handler) };
    match handler.try_borrow_mut() {
        Ok(mut handler) => handler(&mut env, value, arg),
        Err(_) => {
            tracing::warn!("NativeCallback invoked re-entrantly; ignoring");
            0
        }
    }
}
