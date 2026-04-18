//! Packagist API Client
//!
//! Interacts with the Packagist.org registry

use super::super::composer::Autoload;
use super::super::error::Result;
use reqwest::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Packagist API client
pub struct PackagistClient {
    client: Client,
    base_url: String,
}

impl PackagistClient {
    pub fn new(base_url: Option<String>) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::USER_AGENT,
            reqwest::header::HeaderValue::from_static("phprs-pkg/0.1.0"),
        );

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            base_url: base_url.unwrap_or_else(|| "https://repo.packagist.org".to_string()),
        }
    }

    /// Get package metadata from Packagist
    pub async fn get_package_metadata(
        &self,
        package_name: &str,
    ) -> Result<PackageMetadata> {
        let url = format!("{}/p2/{}.json", self.base_url, package_name);

        log::info!("Fetching package metadata from: {}", url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(super::super::error::PkgError::Network)?;

        if !response.status().is_success() {
            return Err(super::super::error::PkgError::PackageNotFound(package_name.to_string()));
        }

        let response_text = response.text().await
            .map_err(super::super::error::PkgError::Network)?;

        let packagist_response: PackagistResponse = serde_json::from_str(&response_text)
            .map_err(|e| {
                log::error!("Failed to parse JSON: {}", e);
                log::debug!("Response text: {}", &response_text[..response_text.len().min(500)]);
                super::super::error::PkgError::Config(format!("JSON parse error: {}", e))
            })?;

        Ok(PackageMetadata::from_packagist(packagist_response))
    }

    /// Download a package distribution
    pub async fn download_package(
        &self,
        package_name: &str,
        version: &str,
        dist: &PackageDist,
        cache_dir: &Path,
    ) -> Result<PathBuf> {
        let package_dir = cache_dir
            .join(package_name.replace('/', "-"))
            .join(version);

        if package_dir.exists() {
            log::info!("Package {}={} already cached", package_name, version);
            return Ok(package_dir);
        }

        log::info!("Downloading {}={} from {}", package_name, version, dist.url);

        // Download archive with redirect support
        let response = self
            .client
            .get(&dist.url)
            .send()
            .await
            .map_err(super::super::error::PkgError::Network)?;

        // Check if we got a successful response
        if !response.status().is_success() {
            return Err(super::super::error::PkgError::ArchiveExtraction(
                format!("Failed to download package: HTTP {}", response.status())
            ));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(super::super::error::PkgError::Network)?;

        log::debug!("Downloaded {} bytes for {}={}", bytes.len(), package_name, version);

        // Verify checksum if provided and non-empty
        if let Some(ref shasum) = dist.shasum {
            if !shasum.is_empty() {
                let computed = format!("{:x}", Sha256::digest(&bytes));
                if computed != *shasum {
                    return Err(super::super::error::PkgError::ChecksumMismatch {
                        package: package_name.to_string(),
                        expected: shasum.clone(),
                        actual: computed,
                    });
                }
                log::debug!("Checksum verified for {}={}", package_name, version);
            } else {
                log::debug!("Skipping checksum verification for {}={} (no checksum provided)", package_name, version);
            }
        }

        // Extract archive
        std::fs::create_dir_all(&package_dir)?;
        self.extract_archive(&bytes, &dist.type_, &package_dir)?;

        log::info!("Extracted {}={} to {}", package_name, version, package_dir.display());

        Ok(package_dir)
    }

    /// Extract archive based on type
    fn extract_archive(&self, bytes: &[u8], archive_type: &str, dest: &Path) -> Result<()> {
        match archive_type {
            "zip" => self.extract_zip(bytes, dest),
            "tar" => self.extract_tar(bytes, dest),
            "gzip" | "tgz" => self.extract_targz(bytes, dest),
            _ => Err(super::super::error::PkgError::ArchiveExtraction(format!(
                "Unsupported archive type: {}",
                archive_type
            ))),
        }
    }

    /// Extract ZIP archive
    fn extract_zip(&self, bytes: &[u8], dest: &Path) -> Result<()> {
        use std::io::Cursor;
        use zip::ZipArchive;

        let reader = Cursor::new(bytes);
        let mut archive = ZipArchive::new(reader)
            .map_err(|e| super::super::error::PkgError::ArchiveExtraction(e.to_string()))?;

        archive
            .extract(dest)
            .map_err(|e| super::super::error::PkgError::ArchiveExtraction(e.to_string()))?;

        Ok(())
    }

    /// Extract TAR archive
    fn extract_tar(&self, bytes: &[u8], dest: &Path) -> Result<()> {
        use std::io::Cursor;
        use tar::Archive;

        let reader = Cursor::new(bytes);
        let mut archive = Archive::new(reader);

        archive
            .unpack(dest)
            .map_err(|e| super::super::error::PkgError::ArchiveExtraction(e.to_string()))?;

        Ok(())
    }

    /// Extract TAR.GZ archive
    fn extract_targz(&self, bytes: &[u8], dest: &Path) -> Result<()> {
        use flate2::read::GzDecoder;
        use std::io::Cursor;
        use tar::Archive;

        let reader = Cursor::new(bytes);
        let decoder = GzDecoder::new(reader);
        let mut archive = Archive::new(decoder);

        archive
            .unpack(dest)
            .map_err(|e| super::super::error::PkgError::ArchiveExtraction(e.to_string()))?;

        Ok(())
    }
}

/// Packagist API response
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct PackagistResponse {
    #[serde(default)]
    minified: Option<String>,
    packages: HashMap<String, Vec<VersionMetadata>>,
    #[serde(default)]
    #[serde(rename = "security-advisories")]
    security_advisories: Option<serde_json::Value>,
}

/// Version metadata from Packagist
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct VersionMetadata {
    pub version: String,
    #[serde(default)]
    pub version_normalized: Option<String>,
    #[serde(default)]
    pub source: Option<PackageSource>,
    #[serde(default)]
    pub dist: Option<PackageDist>,
    #[serde(default)]
    pub require: Option<serde_json::Value>,
    #[serde(default)]
    pub require_dev: Option<serde_json::Value>,
    #[serde(default)]
    pub autoload: Option<Autoload>,
    #[serde(default)]
    pub time: Option<String>,
    #[serde(rename = "type", default)]
    pub type_: Option<String>,
    #[serde(default)]
    pub license: Option<Vec<String>>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub extra: Option<serde_json::Value>,
}

/// Package source (VCS)
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct PackageSource {
    #[serde(rename = "type")]
    pub type_: String,
    pub url: String,
    pub reference: String,
}

/// Package distribution (downloadable archive)
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct PackageDist {
    #[serde(rename = "type")]
    pub type_: String,
    pub url: String,
    pub reference: String,
    pub shasum: Option<String>,
}

/// Package metadata
#[derive(Debug, Clone)]
pub struct PackageMetadata {
    pub name: String,
    pub versions: HashMap<String, VersionMetadata>,
}

impl PackageMetadata {
    fn from_packagist(response: PackagistResponse) -> Self {
        let mut packages = response.packages.into_iter();

        // Get first package (should only be one for a single package query)
        if let Some((name, versions)) = packages.next() {
            let versions_map: HashMap<String, VersionMetadata> = versions
                .into_iter()
                .map(|v| (v.version.clone(), v))
                .collect();

            Self {
                name,
                versions: versions_map,
            }
        } else {
            Self {
                name: String::new(),
                versions: HashMap::new(),
            }
        }
    }

    /// Get the latest stable version
    pub fn latest_stable_version(&self) -> Option<&VersionMetadata> {
        self.versions
            .iter()
            .filter(|(v, _)| !v.contains("dev") && !v.contains("alpha") && !v.contains("beta") && !v.contains("RC"))
            .max_by(|(a, _), (b, _)| {
                // Simple version comparison (should use semver for proper comparison)
                a.cmp(b)
            })
            .map(|(_, meta)| meta)
    }

    /// Get a specific version
    pub fn get_version(&self, version: &str) -> Option<&VersionMetadata> {
        self.versions.get(version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires network access
    async fn test_get_package_metadata() {
        let client = PackagistClient::new(None);
        let metadata = client
            .get_package_metadata("symfony/console")
            .await
            .unwrap();

        assert_eq!(metadata.name, "symfony/console");
        assert!(!metadata.versions.is_empty());
    }
}
