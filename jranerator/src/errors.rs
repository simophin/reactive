use thiserror::Error;

#[derive(Debug, Error)]
pub enum GenerateError {
    #[error("Error reading java class file: {0}")]
    InvalidClassFile(std::io::Error),

    #[error("Invalid jar file: {0}")]
    InvalidJarFile(#[from] zip::result::ZipError),

    #[error("Error writing to destination: {0}")]
    DestinationError(std::io::Error),

    #[error("Error downloading jar: {0}")]
    JarDownloadError(std::io::Error),

    #[error("Error accessing maven: {0}")]
    MavenAccessError(#[from] reqwest::Error),
}
