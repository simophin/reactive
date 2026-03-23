extern crate self as android;

use std::marker::PhantomData;
use std::task::{Context, Waker};

pub use android_macros::view_props;
use jni::objects::JClass;
use jni::sys::jlong;
use jni::JNIEnv;
use reactive_core::ReactiveScope;

/// A static descriptor for a Java setter method, parameterised by the JNI type
/// of the argument.  Holds only `&'static str` data; no allocation, no
/// reflection.  A separate call-site API can use these descriptors to issue
/// type-checked JNI calls.
pub struct PropDescriptor<T> {
    pub class: &'static str,
    pub method: &'static str,
    pub signature: &'static str,
    _marker: PhantomData<fn() -> T>,
}

impl<T> PropDescriptor<T> {
    pub const fn new(class: &'static str, method: &'static str, signature: &'static str) -> Self {
        Self {
            class,
            method,
            signature,
            _marker: PhantomData,
        }
    }
}

// ---------------------------------------------------------------------------
// JNI entrypoints — called by com.reactive.ReactiveScope (Kotlin)
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn Java_com_reactive_ReactiveScope_nativeCreate(
    _env: JNIEnv,
    _class: JClass,
) -> jlong {
    let scope = Box::new(ReactiveScope::default());
    Box::into_raw(scope) as jlong
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    view_props! {
        class "XXX" {
            text: jstring,
            textColor: jint,
            myObj: jobject,
        }
    }

    #[test]
    fn descriptor_class() {
        assert_eq!(TEXT.class, "XXX");
        assert_eq!(TEXT_COLOR.class, "XXX");
        assert_eq!(MY_OBJ.class, "XXX");
    }

    #[test]
    fn descriptor_methods() {
        assert_eq!(TEXT.method, "setText");
        assert_eq!(TEXT_COLOR.method, "setTextColor");
        assert_eq!(MY_OBJ.method, "setMyObj");
    }

    #[test]
    fn descriptor_signatures() {
        assert_eq!(TEXT.signature, "(Ljava/lang/String;)V");
        assert_eq!(TEXT_COLOR.signature, "(I)V");
        assert_eq!(MY_OBJ.signature, "(Ljava/lang/Object;)V");
    }
}
