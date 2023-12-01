use convert_case::{Case, Casing};
use jni::signature::Primitive;
use syn::{parse_quote, TypePath};

pub fn java_primitive_to_rust(primitive: &Primitive) -> TypePath {
    let path: TypePath = match primitive {
        Primitive::Boolean => parse_quote! { jboolean },
        Primitive::Byte => parse_quote! { jbyte },
        Primitive::Char => parse_quote! { jchar },
        Primitive::Double => parse_quote! { jdouble },
        Primitive::Float => parse_quote! { jfloat },
        Primitive::Int => parse_quote! { jint },
        Primitive::Long => parse_quote! { jlong },
        Primitive::Short => parse_quote! { jshort },
        Primitive::Void => parse_quote! { jvoid },
    };

    parse_quote! { ::jni::sys::#path }
}

pub fn java_primitive_array_to_rust(primitive: &Primitive) -> TypePath {
    let path: TypePath = match primitive {
        Primitive::Boolean => parse_quote! { JBooleanArray<'local> },
        Primitive::Byte => parse_quote! { JByteArray<'local> },
        Primitive::Char => parse_quote! { JCharArray<'local> },
        Primitive::Double => parse_quote! { JDoubleArray<'local> },
        Primitive::Float => parse_quote! { JFloatArray<'local> },
        Primitive::Int => parse_quote! { JIntArray<'local> },
        Primitive::Long => parse_quote! { JLongArray<'local> },
        Primitive::Short => parse_quote! { JShortArray<'local> },
        Primitive::Void => panic!("Void arrays are not supported"),
    };

    parse_quote! { ::jni::objects::#path }
}

pub fn java_name_to_rust_name(simple_name: &str) -> String {
    simple_name.to_case(Case::Snake).replace('$', "_")
}
