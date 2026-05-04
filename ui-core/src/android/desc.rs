pub use android_macros::declare_jni_binding;

pub trait JavaClassDescriptor {
    const FQ_NAME: &'static str;
}

pub enum JavaPrimitiveType {
    Void,
    Int,
    Float,
    Boolean,
    Double,
    Char,
    Byte,
}

pub enum JavaReturnType {
    Primitive(JavaPrimitiveType),
    String,
    Object { class_name: &'static str },
}

pub enum JavaFieldType {
    Primitive(JavaPrimitiveType),
    PrimitiveArray(JavaPrimitiveType),
    String,
    Object { class_name: &'static str },
}

pub trait JavaMethodDescriptor<Args> {
    type ClassDescriptor: JavaClassDescriptor;
    const NAME: &'static str;
    const SIGNATURE: &'static str;
    const RETURN_TYPE: JavaReturnType;
}

pub trait JavaFieldDescriptor {
    type ClassDescriptor: JavaClassDescriptor;
    type RustType;
    const NAME: &'static str;
    const SIGNATURE: &'static str;
    const FIELD_TYPE: JavaFieldType;
}
