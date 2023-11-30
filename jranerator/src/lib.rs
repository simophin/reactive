use std::{
    io::{Read, Seek},
    iter::once,
    path::PathBuf,
};

use class_like::ClassLike;
use quote::format_ident;
use syn::{parse_file, token::Pub};
use zip::ZipArchive;

mod class_like;
mod classy_impl;
mod convert;
mod field;
mod method;
mod sig;
mod utils;

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

pub fn generate_jar<'a>(
    archive: &'a mut ZipArchive<impl Read + Seek + 'static>,
) -> impl Iterator<Item = (PathBuf, String)> + 'a {
    (0..archive.len())
        .into_iter()
        .map(move |index| archive.by_index(index).expect("To read file in JAR"))
        .filter(|f| f.is_file() && f.name().ends_with(".class"))
        .map(|file| {
            let path = file
                .enclosed_name()
                .expect("To get file name")
                .to_path_buf();
            let class_file = classy::read_class(file).expect("To read class file");
            let output = convert::convert_class(
                syn::Visibility::Public(Pub::default()),
                format_ident!("{}", class_file.get_simplified_class_name()),
                &class_file,
            );

            (
                path,
                prettyplease::unparse(&parse_file(&output.to_string()).unwrap()),
            )
        })
}
