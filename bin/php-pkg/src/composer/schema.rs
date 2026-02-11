//! Composer.json schema definitions
//!
//! 100% compatible with Composer's composer.json format

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// composer.json structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ComposerJson {
    /// Package name (e.g., "vendor/package")
    pub name: Option<String>,

    /// Package description
    pub description: Option<String>,

    /// Package version (usually managed by VCS, not set manually)
    pub version: Option<String>,

    /// Package type: library, project, metapackage, composer-plugin
    #[serde(rename = "type")]
    pub type_: Option<String>,

    /// Keywords for searching
    pub keywords: Option<Vec<String>>,

    /// Homepage URL
    pub homepage: Option<String>,

    /// License (SPDX identifier or array)
    pub license: Option<StringOrArray>,

    /// Authors
    pub authors: Option<Vec<Author>>,

    /// Support information
    pub support: Option<Support>,

    /// Required packages
    pub require: Option<HashMap<String, String>>,

    /// Development packages
    pub require_dev: Option<HashMap<String, String>>,

    /// Conflict packages
    pub conflict: Option<HashMap<String, String>>,

    /// Replace packages
    pub replace: Option<HashMap<String, String>>,

    /// Provide packages
    pub provide: Option<HashMap<String, String>>,

    /// Suggest packages
    pub suggest: Option<HashMap<String, String>>,

    /// Autoloading rules
    pub autoload: Option<Autoload>,

    /// Development autoloading rules
    pub autoload_dev: Option<Autoload>,

    /// Target directory for autoloader
    pub target_dir: Option<String>,

    /// Minimum stability (stable, RC, beta, alpha, dev)
    pub minimum_stability: Option<String>,

    /// Prefer stable packages
    pub prefer_stable: Option<bool>,

    /// Repositories (custom package sources)
    pub repositories: Option<Vec<Repository>>,

    /// Configuration options
    pub config: Option<Config>,

    /// Scripts to run
    pub scripts: Option<HashMap<String, ScriptDefinition>>,

    /// Extra data (for plugins)
    pub extra: Option<serde_json::Value>,

    /// Binary files to install
    pub bin: Option<Vec<String>>,

    /// Archive configuration
    pub archive: Option<Archive>,

    /// Abandoned package notice
    pub abandoned: Option<bool>,

    /// Non-feature branches
    pub non_feature_branches: Option<Vec<String>>,
}

/// Autoloading configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Autoload {
    /// PSR-4 autoloading
    #[serde(rename = "psr-4")]
    pub psr_4: Option<HashMap<String, StringOrArray>>,

    /// PSR-0 autoloading (deprecated)
    #[serde(rename = "psr-0")]
    pub psr_0: Option<HashMap<String, StringOrArray>>,

    /// Classmap autoloading
    pub classmap: Option<Vec<String>>,

    /// Files to include
    pub files: Option<Vec<String>>,

    /// Exclude from classmap
    pub exclude_from_classmap: Option<Vec<String>>,
}

/// Author information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Author {
    pub name: Option<String>,
    pub email: Option<String>,
    pub homepage: Option<String>,
    pub role: Option<String>,
}

/// Support information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Support {
    pub email: Option<String>,
    pub issues: Option<String>,
    pub forum: Option<String>,
    pub wiki: Option<String>,
    pub irc: Option<String>,
    pub source: Option<String>,
    pub docs: Option<String>,
    pub rss: Option<String>,
    pub chat: Option<String>,
}

/// Repository definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Repository {
    /// Composer repository
    Composer {
        url: String,
        #[serde(default)]
        canonical: Option<bool>,
        #[serde(default)]
        only: Option<Vec<String>>,
        #[serde(default)]
        exclude: Option<Vec<String>>,
    },

    /// VCS repository (git, hg, svn)
    Vcs {
        url: String,
        #[serde(default)]
        no_api: Option<bool>,
    },

    /// Path repository (local)
    Path { url: String, options: Option<PathOptions> },

    /// Package repository (inline)
    Package { package: ComposerJson },

    /// Artifact repository (local zip files)
    Artifact { url: String },
}

/// Path repository options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathOptions {
    #[serde(default)]
    pub symlink: Option<bool>,
    #[serde(default)]
    pub relative: Option<bool>,
    #[serde(default)]
    pub versions: Option<HashMap<String, String>>,
}

/// Configuration options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    pub vendor_dir: Option<String>,
    pub bin_dir: Option<String>,
    pub optimize_autoloader: Option<bool>,
    pub preferred_install: Option<String>,
    pub sort_packages: Option<bool>,
    pub check_platform_packages: Option<bool>,
    pub use_include_path: Option<bool>,
    pub platform: Option<HashMap<String, String>>,
    pub allow_plugins: Option<AllowPlugins>,
}

/// Allow plugins configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AllowPlugins {
    Simple(bool),
    Complex {
        #[serde(default)]
        packagist: Option<bool>,
        #[serde(default)]
        symfony: Option<bool>,
    },
}

/// Script definition (string or array of strings)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ScriptDefinition {
    Simple(String),
    Complex(Vec<String>),
}

/// String or array of strings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StringOrArray {
    Single(String),
    Multiple(Vec<String>),
}

/// Archive configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Archive {
    #[serde(default)]
    pub exclude: Option<Vec<String>>,
}

impl ComposerJson {
    /// Load composer.json from file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(crate::error::PkgError::ComposerJsonRead)?;

        let composer: ComposerJson = serde_json::from_str(&content)
            .map_err(crate::error::PkgError::ComposerJsonParse)?;

        Ok(composer)
    }

    /// Save composer.json to file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| crate::error::PkgError::Config(e.to_string()))?;

        std::fs::write(path.as_ref(), content)
            .map_err(crate::error::PkgError::ComposerJsonWrite)?;

        Ok(())
    }

    /// Create new composer.json with defaults
    pub fn new(name: Option<String>) -> Self {
        Self {
            name,
            description: None,
            version: None,
            type_: Some("project".to_string()),
            keywords: None,
            homepage: None,
            license: None,
            authors: None,
            support: None,
            require: Some(HashMap::new()),
            require_dev: Some(HashMap::new()),
            conflict: None,
            replace: None,
            provide: None,
            suggest: None,
            autoload: Some(Autoload::default()),
            autoload_dev: None,
            target_dir: None,
            minimum_stability: Some("stable".to_string()),
            prefer_stable: Some(true),
            repositories: None,
            config: None,
            scripts: None,
            extra: None,
            bin: None,
            archive: None,
            abandoned: None,
            non_feature_branches: None,
        }
    }

    /// Add a requirement
    pub fn add_require(&mut self, package: String, version: String) {
        if let Some(ref mut require) = self.require {
            require.insert(package, version);
        } else {
            let mut require = HashMap::new();
            require.insert(package, version);
            self.require = Some(require);
        }
    }
}

impl Default for Autoload {
    fn default() -> Self {
        Self {
            psr_4: Some(HashMap::new()),
            psr_0: None,
            classmap: None,
            files: None,
            exclude_from_classmap: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_composer_json() {
        let json = r#"{
            "name": "test/package",
            "description": "A test package",
            "type": "library",
            "require": {
                "php": "^8.0"
            },
            "autoload": {
                "psr-4": {
                    "Test\\": "src/"
                }
            }
        }"#;

        let composer: ComposerJson = serde_json::from_str(json).unwrap();
        assert_eq!(composer.name, Some("test/package".to_string()));
        assert_eq!(composer.description, Some("A test package".to_string()));
        assert!(composer.require.is_some());
    }
}
