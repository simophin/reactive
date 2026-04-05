pub mod desc;

extern crate self as android;

use std::task::{Context, Waker};

pub use android_macros::declare_jni_binding;
pub use desc::{
    JavaClassDescriptor, JavaFieldDescriptor, JavaFieldType, JavaMethodDescriptor,
    JavaPrimitiveType, JavaReturnType,
};
use jni::objects::JClass;
use jni::sys::jlong;
use jni::JNIEnv;
use reactive_core::ReactiveScope;

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
    use jni::sys::{jbyteArray, jint, jstring};

    declare_jni_binding! {
        class com.mypackage.MyClass {
            int age;
            String label;
            byte[] payload;
            String getText();
            char getInitial();
            byte getTag();
            void setAge(int);
            void setAge(int, int);
        }
    }

    #[test]
    fn declared_jni_binding_class_descriptor() {
        assert_eq!(
            <MyClass as JavaClassDescriptor>::FQ_NAME,
            "com/mypackage/MyClass"
        );
    }

    #[test]
    fn declared_jni_binding_method_signatures() {
        assert_eq!(
            <getText as JavaMethodDescriptor<()>>::SIGNATURE,
            "()Ljava/lang/String;"
        );
        assert_eq!(<getInitial as JavaMethodDescriptor<()>>::SIGNATURE, "()C");
        assert_eq!(<getTag as JavaMethodDescriptor<()>>::SIGNATURE, "()B");
        assert_eq!(<setAge as JavaMethodDescriptor<(jint,)>>::SIGNATURE, "(I)V");
        assert_eq!(
            <setAge as JavaMethodDescriptor<(jint, jint)>>::SIGNATURE,
            "(II)V"
        );
    }

    #[test]
    fn declared_jni_binding_method_class_descriptors() {
        fn assert_method_class_descriptor<M, Args>()
        where
            M: JavaMethodDescriptor<Args, ClassDescriptor = MyClass>,
        {
        }

        assert_method_class_descriptor::<getText, ()>();
        assert_method_class_descriptor::<getInitial, ()>();
        assert_method_class_descriptor::<getTag, ()>();
        assert_method_class_descriptor::<setAge, (jint,)>();
        assert_method_class_descriptor::<setAge, (jint, jint)>();
    }

    #[test]
    fn declared_jni_binding_method_return_types() {
        assert!(matches!(
            <getText as JavaMethodDescriptor<()>>::RETURN_TYPE,
            JavaReturnType::String
        ));
        assert!(matches!(
            <getInitial as JavaMethodDescriptor<()>>::RETURN_TYPE,
            JavaReturnType::Primitive(JavaPrimitiveType::Char)
        ));
        assert!(matches!(
            <getTag as JavaMethodDescriptor<()>>::RETURN_TYPE,
            JavaReturnType::Primitive(JavaPrimitiveType::Byte)
        ));
        assert!(matches!(
            <setAge as JavaMethodDescriptor<(jint,)>>::RETURN_TYPE,
            JavaReturnType::Primitive(JavaPrimitiveType::Void)
        ));
        assert!(matches!(
            <setAge as JavaMethodDescriptor<(jint, jint)>>::RETURN_TYPE,
            JavaReturnType::Primitive(JavaPrimitiveType::Void)
        ));
    }

    #[test]
    fn declared_jni_binding_field_signatures() {
        assert_eq!(<age as JavaFieldDescriptor>::SIGNATURE, "I");
        assert_eq!(
            <label as JavaFieldDescriptor>::SIGNATURE,
            "Ljava/lang/String;"
        );
        assert_eq!(<payload as JavaFieldDescriptor>::SIGNATURE, "[B");
    }

    #[test]
    fn declared_jni_binding_field_types() {
        assert!(matches!(
            <age as JavaFieldDescriptor>::FIELD_TYPE,
            JavaFieldType::Primitive(JavaPrimitiveType::Int)
        ));
        assert!(matches!(
            <label as JavaFieldDescriptor>::FIELD_TYPE,
            JavaFieldType::String
        ));
        assert!(matches!(
            <payload as JavaFieldDescriptor>::FIELD_TYPE,
            JavaFieldType::PrimitiveArray(JavaPrimitiveType::Byte)
        ));
    }

    #[test]
    fn declared_jni_binding_field_class_descriptors_and_rust_types() {
        fn assert_field_descriptor<F, T>()
        where
            F: JavaFieldDescriptor<ClassDescriptor = MyClass, RustType = T>,
        {
        }

        assert_field_descriptor::<age, jint>();
        assert_field_descriptor::<label, jstring>();
        assert_field_descriptor::<payload, jbyteArray>();
    }
}
