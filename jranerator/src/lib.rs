use std::io::Read;

use quote::format_ident;
use syn::{parse_file, token::Pub};

mod class_like;
mod classy_impl;
mod convert;
mod sig;

pub fn generate(class_file: impl Read, name: &str) -> String {
    let class_file = classy::read_class(class_file).expect("To read class file");

    let output = convert::convert_class(
        syn::Visibility::Public(Pub::default()),
        format_ident!("{}", name),
        &class_file,
    );
    prettyplease::unparse(&parse_file(&output.to_string()).unwrap())
}
