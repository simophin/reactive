use std::{
    fs::{create_dir_all, remove_dir_all},
    path::PathBuf,
};

use jranerator::{generate_from_maven, Module};

fn main() {
    let mut root_module = Module::new("binding".to_string());

    generate_from_maven(
        "https://repo1.maven.org/maven2",
        "com.google.code.gson",
        "gson",
        "2.10.1",
        "jar",
        &mut root_module,
    )
    .expect("To generate module");

    let output = PathBuf::from(&root_module.name);
    let _ = remove_dir_all(&output);
    create_dir_all(&output).expect("To create output directory");

    root_module
        .write_to(&output)
        .expect("To write to output directory");
}
