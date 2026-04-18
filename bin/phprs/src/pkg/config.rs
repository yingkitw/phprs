//! Configuration management for pkg

use super::error::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Vendor directory (default: "vendor")
    pub vendor_dir: PathBuf,

    /// Cache directory for downloaded packages
    pub cache_dir: PathBuf,

    /// Packagist registry URL
    pub registry_url: String,

    /// Number of parallel downloads
    pub parallel_downloads: usize,

    /// Enable verbose output
    pub verbose: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            vendor_dir: PathBuf::from("vendor"),
            cache_dir: dirs::cache_dir()
                .unwrap_or_else(|| PathBuf::from(".cache"))
                .join("php-pkg"),
            registry_url: "https://repo.packagist.org".to_string(),
            parallel_downloads: 5,
            verbose: false,
        }
    }
}

impl Config {
    /// Load configuration from file, or return defaults
    pub fn load(project_dir: &Path) -> Result<Self> {
        let config_file = project_dir.join("php-pkg.toml");

        if config_file.exists() {
            let content = std::fs::read_to_string(&config_file)?;
            let config: Config = toml::from_str(&content)
                .map_err(|e| super::error::PkgError::Config(e.to_string()))?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    /// Get the cache directory for package metadata
    #[allow(dead_code)]
    pub fn metadata_cache_dir(&self) -> PathBuf {
        self.cache_dir.join("metadata")
    }

    /// Get the cache directory for downloaded packages
    pub fn packages_cache_dir(&self) -> PathBuf {
        self.cache_dir.join("packages")
    }

    /// Get the vendor directory (relative to project root)
    pub fn vendor_dir(&self) -> &Path {
        &self.vendor_dir
    }

    /// Get the autoloader path
    #[allow(dead_code)]
    pub fn autoloader_path(&self) -> PathBuf {
        self.vendor_dir.join("autoload.php")
    }
}
