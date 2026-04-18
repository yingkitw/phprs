//! Error types for pkg

use thiserror::Error;

#[derive(Error, Debug)]
pub enum PkgError {
    #[error("Failed to read composer.json: {0}")]
    ComposerJsonRead(#[source] std::io::Error),

    #[error("Failed to parse composer.json: {0}")]
    ComposerJsonParse(#[source] serde_json::Error),

    #[error("Failed to write composer.json: {0}")]
    ComposerJsonWrite(#[source] std::io::Error),

    #[error("Package not found: {0}")]
    PackageNotFound(String),

    #[error("Version not found for package {package}: {version}")]
    VersionNotFound { package: String, version: String },

    #[error("Dependency resolution failed: {0}")]
    #[allow(dead_code)]
    DependencyResolution(String),

    #[error("Network error: {0}")]
    Network(#[source] reqwest::Error),

    #[error("Archive extraction failed: {0}")]
    ArchiveExtraction(String),

    #[error("Checksum mismatch for {package}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        package: String,
        expected: String,
        actual: String,
    },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, PkgError>;
