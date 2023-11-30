use std::io::Cursor;

use crate::{generate_from_jar, GenerateError, Module};

pub fn generate_from_maven(
    repo: &str,
    group_id: &str,
    artifact_id: &str,
    version: &str,
    package_type_suffix: &str,
    root_module: &mut Module,
) -> Result<(), GenerateError> {
    let jar_contents = download_jar(repo, group_id, artifact_id, version, package_type_suffix)?;

    generate_from_jar(Cursor::new(jar_contents), root_module)
}

fn download_jar(
    repo: &str,
    group_id: &str,
    artifact_id: &str,
    version: &str,
    package_type_suffix: &str,
) -> Result<Vec<u8>, GenerateError> {
    let group_id = group_id.replace(".", "/");
    let url = format!(
        "{repo}/{group_id}/{artifact_id}/{version}/{artifact_id}-{version}.{package_type_suffix}"
    );

    let mut response = reqwest::blocking::get(&url)?.error_for_status()?;
    let mut contents = Vec::new();
    response.copy_to(&mut contents)?;

    Ok(contents)
}
