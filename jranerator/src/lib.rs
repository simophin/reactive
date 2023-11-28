use std::io::Read;

use class_like::ClassLike;
use quote::format_ident;
use syn::{parse_file, token::Pub};

mod class_like;
mod classy_impl;
mod convert;
mod sig;

pub fn generate(class_file: impl Read, name: Option<&str>) -> String {
    let class_file = classy::read_class(class_file).expect("To read class file");

    let output = convert::convert_class(
        syn::Visibility::Public(Pub::default()),
        format_ident!(
            "{}",
            name.map(|n| n.to_string())
                .unwrap_or_else(|| class_file.get_simplified_class_name())
        ),
        &class_file,
    );
    prettyplease::unparse(&parse_file(&output.to_string()).unwrap())
}
