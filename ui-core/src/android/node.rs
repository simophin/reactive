use jni::objects::{GlobalRef, JObject, JValue};
use jni::{JNIEnv, JavaVM};
use std::sync::OnceLock;

static JAVA_VM: OnceLock<JavaVM> = OnceLock::new();

pub(crate) fn init_java_vm(env: &JNIEnv) {
    let _ = JAVA_VM.set(env.get_java_vm().expect("get JavaVM"));
}

/// The JNI environment of the current thread, attaching the thread to the JVM if needed.
pub fn env() -> JNIEnv<'static> {
    JAVA_VM
        .get()
        .expect("JavaVM not initialised; is the app declared with android_main!?")
        .attach_current_thread_permanently()
        .expect("attach thread to JVM")
}

/// Panics with the pending Java exception, if `result` failed.
#[track_caller]
pub(crate) fn check<T>(env: &mut JNIEnv, result: jni::errors::Result<T>, what: &str) -> T {
    match result {
        Ok(value) => value,
        Err(err) => {
            if env.exception_check().unwrap_or(false) {
                let _ = env.exception_describe();
                let _ = env.exception_clear();
            }
            panic!("JNI call `{what}` failed: {err}");
        }
    }
}

/// A JVM object owned from Rust, usually a Kotlin `com.reactive.Node`.
///
/// Equality is identity of the underlying global reference, which clones share.
#[derive(Clone)]
pub struct Node(GlobalRef);

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_obj().as_raw() == other.0.as_obj().as_raw()
    }
}

impl Eq for Node {}

impl Node {
    /// Constructs `class` (JNI name, e.g. `com/reactive/TextNode`) with its no-arg constructor.
    pub fn new(class: &str) -> Self {
        Self::new_with(class, "()V", &[])
    }

    pub fn new_with(class: &str, ctor_sig: &str, args: &[JValue]) -> Self {
        let mut env = env();
        let obj = env.new_object(class, ctor_sig, args);
        let obj = check(&mut env, obj, class);
        Self::from_local(&mut env, obj)
    }

    pub fn from_local(env: &mut JNIEnv, obj: JObject) -> Self {
        let global = env.new_global_ref(&obj);
        let global = check(env, global, "new_global_ref");
        let _ = env.delete_local_ref(obj);
        Self(global)
    }

    pub fn as_obj(&self) -> &JObject<'static> {
        self.0.as_obj()
    }

    /// Calls a single-argument `void` method, typically a Kotlin property setter.
    pub fn set<A: JniArg>(&self, method: &str, value: A) {
        let mut env = env();
        let sig = format!("({})V", A::SIG);
        let result = value.with_jvalue(&mut env, |env, value| {
            env.call_method(self.as_obj(), method, &sig, &[value])
                .map(|_| ())
        });
        check(&mut env, result, method);
    }

    /// Calls a method with an explicit signature.
    pub fn call<'a>(
        &self,
        method: &str,
        sig: &str,
        args: &[JValue],
    ) -> jni::objects::JValueOwned<'a> {
        let mut env = env();
        let result = env.call_method(self.as_obj(), method, sig, args);
        check(&mut env, result, method)
    }
}

/// A Rust value that can be passed as a single JNI argument.
pub trait JniArg {
    /// JNI type signature, e.g. `F` or `Ljava/lang/String;`.
    const SIG: &'static str;

    fn with_jvalue<R>(&self, env: &mut JNIEnv, f: impl FnOnce(&mut JNIEnv, JValue) -> R) -> R;
}

macro_rules! primitive_arg {
    ($ty:ty, $sig:literal, |$v:ident| $value:expr) => {
        impl JniArg for $ty {
            const SIG: &'static str = $sig;

            fn with_jvalue<R>(
                &self,
                env: &mut JNIEnv,
                f: impl FnOnce(&mut JNIEnv, JValue) -> R,
            ) -> R {
                let $v = *self;
                f(env, $value)
            }
        }
    };
}

primitive_arg!(bool, "Z", |v| JValue::Bool(v as u8));
primitive_arg!(i32, "I", |v| JValue::Int(v));
primitive_arg!(i64, "J", |v| JValue::Long(v));
primitive_arg!(f32, "F", |v| JValue::Float(v));

impl JniArg for &str {
    const SIG: &'static str = "Ljava/lang/String;";

    fn with_jvalue<R>(&self, env: &mut JNIEnv, f: impl FnOnce(&mut JNIEnv, JValue) -> R) -> R {
        let string = env.new_string(self);
        let string = check(env, string, "new_string");
        let string = env.auto_local(string);
        f(env, JValue::Object(&string))
    }
}

impl JniArg for Option<&str> {
    const SIG: &'static str = "Ljava/lang/String;";

    fn with_jvalue<R>(&self, env: &mut JNIEnv, f: impl FnOnce(&mut JNIEnv, JValue) -> R) -> R {
        match self {
            Some(s) => s.with_jvalue(env, f),
            None => f(env, JValue::Object(&JObject::null())),
        }
    }
}

/// A node passed where Kotlin expects the `com.reactive.Node` base type.
impl JniArg for &Node {
    const SIG: &'static str = "Lcom/reactive/Node;";

    fn with_jvalue<R>(&self, env: &mut JNIEnv, f: impl FnOnce(&mut JNIEnv, JValue) -> R) -> R {
        f(env, JValue::Object(self.as_obj()))
    }
}

impl JniArg for Option<&Node> {
    const SIG: &'static str = "Lcom/reactive/Node;";

    fn with_jvalue<R>(&self, env: &mut JNIEnv, f: impl FnOnce(&mut JNIEnv, JValue) -> R) -> R {
        let null = JObject::null();
        f(env, JValue::Object(self.map(Node::as_obj).unwrap_or(&null)))
    }
}
