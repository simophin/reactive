use proc_macro2::Ident;

use crate::sig::JavaTypeDescription;

pub struct JavaField {
    pub desc: JavaTypeDescription,
    pub rust_field_name: Ident,
}

impl JavaField {
    // pub fn from(java_class: &impl ClassLike) -> Vec<JavaField> {
    //     java_class
    //         .get_public_fields()
    //         .iter()
    //         .map(|f| JavaField {
    //             desc: f.signature.parse().expect("A valid Java field signature"),
    //             rust_field_name: Ident::new(&f.get_name().to_case(Case::Snake), Span::call_site()),
    //         })
    //         .collect()
    // }
}
