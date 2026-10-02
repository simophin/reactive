//! Thin helpers over the `jni` crate.
//!
//! All UI work happens on the Android main thread, which is always attached to
//! the JVM, so [`env`] is a cheap `GetEnv` call rather than an attach.

use jni::errors::{Error, Result};
use jni::objects::{GlobalRef, JObject, JValue, JValueOwned};
use jni::{JNIEnv, JavaVM};
use std::sync::OnceLock;

static JAVA_VM: OnceLock<JavaVM> = OnceLock::new();

pub(crate) fn init_vm(vm: JavaVM) {
    let _ = JAVA_VM.set(vm);
}

pub(crate) fn vm() -> &'static JavaVM {
    JAVA_VM
        .get()
        .expect("JavaVM not initialised: was `android_main!` used?")
}

/// The JNI environment for the current thread, attaching it if necessary.
pub(crate) fn env() -> JNIEnv<'static> {
    vm().get_env()
        .or_else(|_| vm().attach_current_thread_permanently())
        .expect("attach current thread to the JVM")
}

/// Unwraps a JNI result. A pending Java exception is printed to logcat and
/// cleared before panicking, so it never leaks into unrelated JNI calls.
pub(crate) fn check<T>(env: &JNIEnv, result: Result<T>, what: &str) -> T {
    match result {
        Ok(value) => value,
        Err(err) => {
            if matches!(err, Error::JavaException) {
                let _ = env.exception_describe();
                let _ = env.exception_clear();
            }
            panic!("JNI call failed ({what}): {err}");
        }
    }
}

/// A global reference to a Java object.
///
/// Equality is reference identity of the underlying `GlobalRef`: clones of the
/// same `JavaObject` compare equal, two separately created global refs to the
/// same Java object do not. This is all the view registries need and avoids a
/// JNI round-trip per comparison.
#[derive(Clone, Debug)]
pub struct JavaObject(GlobalRef);

impl PartialEq for JavaObject {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_obj().as_raw() == other.0.as_obj().as_raw()
    }
}

impl Eq for JavaObject {}

impl JavaObject {
    pub fn new(env: &JNIEnv, obj: &JObject) -> Self {
        Self(check(env, env.new_global_ref(obj), "new_global_ref"))
    }

    /// Instantiates `class` with the given constructor.
    pub fn construct(class: &str, sig: &str, args: &[JValue]) -> Self {
        let mut env = env();
        let obj = env.new_object(class, sig, args);
        let obj = check(&env, obj, class);
        let this = Self::new(&env, &obj);
        let _ = env.delete_local_ref(obj);
        this
    }

    pub fn as_obj(&self) -> &JObject<'static> {
        self.0.as_obj()
    }

    pub fn call(&self, name: &str, sig: &str, args: &[JValue]) -> JValueOwned<'static> {
        let mut env = env();
        let result = env.call_method(self.as_obj(), name, sig, args);
        check(&env, result, name)
    }

    pub fn call_void(&self, name: &str, sig: &str, args: &[JValue]) {
        self.call(name, sig, args);
    }

    pub fn call_int(&self, name: &str) -> i32 {
        self.call(name, "()I", &[]).i().expect("int return")
    }

    /// Calls a method returning an object and wraps it in a global ref.
    pub fn call_object(&self, name: &str, sig: &str, args: &[JValue]) -> Option<JavaObject> {
        let obj = self.call(name, sig, args).l().expect("object return");
        if obj.is_null() {
            return None;
        }
        let env = env();
        let result = Self::new(&env, &obj);
        let _ = env.delete_local_ref(obj);
        Some(result)
    }

    pub fn set_long_field(&self, name: &str, value: i64) {
        let mut env = env();
        let result = env.set_field(self.as_obj(), name, "J", JValue::Long(value));
        check(&env, result, name);
    }

    pub fn is_same(&self, other: &JObject) -> bool {
        let env = env();
        let result = env.is_same_object(self.as_obj(), other);
        check(&env, result, "is_same_object")
    }

    /// Calls a `(CharSequence)V` setter such as `TextView.setText`.
    pub fn set_text(&self, method: &str, text: &str) {
        let env = env();
        let string = check(&env, env.new_string(text), "new_string");
        self.call_void(
            method,
            "(Ljava/lang/CharSequence;)V",
            &[JValue::Object(&string)],
        );
        let _ = env.delete_local_ref(string);
    }
}

impl JavaObject {
    pub fn get_float_field(&self, name: &str) -> f32 {
        let mut env = env();
        let result = env.get_field(self.as_obj(), name, "F").and_then(|v| v.f());
        check(&env, result, name)
    }
}
