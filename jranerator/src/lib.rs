use std::{
    io::{Read, Seek},
    path::Path,
};

use class_like::ClassLike;
use quote::format_ident;
use syn::{parse_file, token::Pub};
use zip::ZipArchive;
use thiserror::Error;

mod class_like;
mod classy_impl;
mod convert;
mod field;
mod method;
mod sig;
mod utils;

#[derive(Debug, Error)]
pub enum GenerateError {
    #[error("Error reading java class file: {0}")]
    InvalidClassFile(#[from] std::io::Error),

    #[error("Invalid jar file: {0}")]
    InvalidJarFile(#[from] zip::result::ZipError),
}

pub fn generate(class_file: impl Read, name: Option<&str>) -> Result<String, GenerateError> {
    let class_file = classy::read_class(class_file)?;

    let output = convert::convert_class(
        syn::Visibility::Public(Pub::default()),
        format_ident!(
            "{}",
            name.map(|n| n.to_string())
                .unwrap_or_else(|| class_file.get_simplified_class_name())
        ),
        &class_file,
    );

    Ok(prettyplease::unparse(&parse_file(&output.to_string()).unwrap()))
}

pub fn generate_jar(
    archive: impl Read + Seek,
    mut for_each: impl FnMut(&Path, String) -> (),
) -> Result<(), GenerateError> {
    let mut archive = ZipArchive::new(archive)?;
    for i in 0..archive.len() {
        let file = archive.by_index(i).expect("To read file in JAR");
        if file.is_file() && file.name().ends_with(".class") {
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

            for_each(
                path.as_path(),
                prettyplease::unparse(&parse_file(&output.to_string()).unwrap()),
            );
        }
    }

    Ok(())
}
