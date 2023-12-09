use std::{
    fs::{create_dir_all, remove_dir_all},
    path::PathBuf,
};

use jranerator::{generate_from_maven, Module};

fn main() {
    let mut root_module = Module::new("binding".to_string());

    generate_from_maven(
        jranerator::Repository::Google,
        "androidx.autofill:autofill:1.1.0".try_into().unwrap(),
        &mut root_module,
    )
    .expect("To generate module");

    generate_from_maven(
        jranerator::Repository::MavenCentral,
        "com.google.android:android:4.1.1.4".try_into().unwrap(),
        &mut root_module,
    )
    .expect("To generate module");

    let output = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("src")
        .join(&root_module.name);
    let _ = remove_dir_all(&output);
    create_dir_all(&output).expect("To create output directory");

    root_module
        .write_to(&output)
        .expect("To write to output directory");
}
