use std::{
    borrow::Cow,
    io::{BufReader, Cursor, Read},
};

use zip::ZipArchive;

use crate::{generate_from_jar, GenerateError, Module};

pub enum Repository<'a> {
    MavenCentral,
    Google,
    Custom(&'a str),
}

impl Repository<'_> {
    pub fn url(&self) -> &str {
        match self {
            Repository::MavenCentral => "https://repo1.maven.org/maven2",
            Repository::Google => "https://maven.google.com",
            Repository::Custom(url) => url,
        }
    }
}

pub struct VersionSpec<'a> {
    pub group_id: &'a str,
    pub artifact_id: &'a str,
    pub version: &'a str,
}

impl<'a> TryFrom<&'a str> for VersionSpec<'a> {
    type Error = &'a str;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let mut parts = value.split(':');
        let group_id = parts.next().ok_or("Missing group id")?;
        let artifact_id = parts.next().ok_or("Missing artifact id")?;
        let version = parts.next().ok_or("Missing version")?;

        Ok(VersionSpec {
            group_id,
            artifact_id,
            version,
        })
    }
}

pub fn generate_from_maven(
    repo: Repository<'_>,
    spec: VersionSpec<'_>,
    root_module: &mut Module,
) -> Result<(), GenerateError> {
    let pom_contents = download_maven(repo.url(), &spec, "pom")?;
    let pom = xmltree::Element::parse(Cursor::new(pom_contents))?;
    let jar_contents = match pom
        .get_child("packaging")
        .and_then(|e| e.get_text())
        .unwrap_or(Cow::Borrowed("jar"))
        .as_ref()
    {
        "aar" => {
            let aar_contents = download_maven(repo.url(), &spec, "aar")?;
            let mut aar_contents = ZipArchive::new(Cursor::new(aar_contents))?;
            let mut buf = Vec::new();
            BufReader::new(aar_contents.by_name("classes.jar")?)
                .read_to_end(&mut buf)
                .map_err(|e| GenerateError::JarDownloadError(e))?;
            buf
        }

        "jar" => download_maven(repo.url(), &spec, "jar")?,

        other => {
            return Err(GenerateError::InvalidPOMPackaging(other.to_string()));
        }
    };

    generate_from_jar(Box::new(Cursor::new(jar_contents)), root_module)
}

fn download_maven(
    repo: &str,
    spec: &VersionSpec<'_>,
    package_type_suffix: &str,
) -> Result<Vec<u8>, GenerateError> {
    let VersionSpec {
        group_id,
        artifact_id,
        version,
    } = spec;
    let group_id = group_id.replace(".", "/");
    let url = format!(
        "{repo}/{group_id}/{artifact_id}/{version}/{artifact_id}-{version}.{package_type_suffix}"
    );

    let mut response = reqwest::blocking::get(&url)?.error_for_status()?;
    let mut contents = Vec::new();
    response.copy_to(&mut contents)?;

    Ok(contents)
}
