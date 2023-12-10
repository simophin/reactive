use convert_case::{Case, Casing};

pub fn java_name_to_rust_name(simple_name: &str) -> String {
    simple_name.to_case(Case::Snake).replace('$', "_")
}
