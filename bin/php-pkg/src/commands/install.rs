//! Install command - Install dependencies from composer.json

use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct Install {
    /// Project directory (default: current directory)
    #[arg(short, long)]
    path: Option<PathBuf>,

    /// Install development dependencies
    #[arg(long)]
    no_dev: bool,

    /// Optimize autoloader
    #[arg(long)]
    optimize_autoloader: bool,

    /// Prefer dist packages
    #[arg(long)]
    prefer_dist: bool,

    /// Prefer source packages
    #[arg(long)]
    prefer_source: bool,
}

impl Install {
    pub async fn execute(&self) -> anyhow::Result<()> {
        let project_dir = self.path.as_ref()
            .unwrap_or(&PathBuf::from("."))
            .canonicalize()?;

        log::info!("Installing dependencies in: {}", project_dir.display());

        // Load composer.json
        let composer_json_path = project_dir.join("composer.json");
        if !composer_json_path.exists() {
            return Err(anyhow::anyhow!(
                "composer.json not found in {}",
                project_dir.display()
            ));
        }

        let composer = crate::composer::ComposerJson::load(&composer_json_path)?;
        log::info!("Loaded composer.json: {:?}", composer.name);

        // Load configuration
        let config = crate::config::Config::load(&project_dir)?;

        // Create vendor directory
        let vendor_dir = project_dir.join(config.vendor_dir());
        if !vendor_dir.exists() {
            std::fs::create_dir_all(&vendor_dir)?;
            log::info!("✓ Created vendor directory");
        }

        // Create autoload cache directory
        std::fs::create_dir_all(&config.cache_dir)?;

        // Phase 1: Install packages without dependency resolution
        if let Some(ref require) = composer.require {
            let client = crate::registry::PackagistClient::new(Some(config.registry_url.clone()));

            log::info!("Installing {} package(s)...", require.len());

            for (package_name, version_constraint) in require {
                // Skip PHP version requirement
                if package_name == "php" {
                    continue;
                }

                log::info!("Installing {} ({})...", package_name, version_constraint);

                match self.install_package(&client, package_name, version_constraint, &vendor_dir, &config).await {
                    Ok(_) => log::info!("✓ Installed {}", package_name),
                    Err(e) => log::error!("Failed to install {}: {}", package_name, e),
                }
            }
        }

        // Generate autoloader
        self.generate_autoloader(&composer, &vendor_dir)?;

        log::info!("");
        log::info!("✨ Installation complete!");

        Ok(())
    }

    /// Install a single package
    async fn install_package(
        &self,
        client: &crate::registry::PackagistClient,
        package_name: &str,
        version_constraint: &str,
        vendor_dir: &std::path::Path,
        config: &crate::config::Config,
    ) -> anyhow::Result<()> {
        // Fetch package metadata
        let metadata = client.get_package_metadata(package_name).await?;

        // For Phase 1, just use the latest stable version
        // TODO: Phase 2 - Parse version constraints and resolve properly
        let version_meta = metadata.latest_stable_version()
            .ok_or_else(|| anyhow::anyhow!("No stable version found for {}", package_name))?;

        let version = version_meta.version.clone();
        log::debug!("Selected version: {}", version);

        // Check if dist is available
        let dist = version_meta.dist.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No dist available for {} {}", package_name, version))?;

        // Download and extract package
        let package_dir = client.download_package(
            package_name,
            &version,
            dist,
            &config.packages_cache_dir(),
        ).await?;

        // Copy to vendor directory
        let target_dir = vendor_dir.join(package_name.replace('/', "/"));
        if !target_dir.exists() {
            // For zip files, the extracted content is usually in a subdirectory
            // We need to find it and copy to vendor
            let entries: Vec<_> = std::fs::read_dir(&package_dir)?.collect::<Result<Vec<_>, _>>()?;
            if entries.len() == 1 {
                // Single directory (typical for GitHub archives)
                let extracted_dir = entries[0].path();
                std::fs::create_dir_all(target_dir.parent().unwrap())?;
                crate::composer::copy_dir_all(&extracted_dir, &target_dir)?;
            } else {
                // Multiple files/directories at root
                std::fs::create_dir_all(&target_dir)?;
                crate::composer::copy_dir_all(&package_dir, &target_dir)?;
            }
        }

        Ok(())
    }

    /// Generate PSR-4 autoloader
    fn generate_autoloader(
        &self,
        composer: &crate::composer::ComposerJson,
        vendor_dir: &std::path::Path,
    ) -> anyhow::Result<()> {
        let mut autoload_mappings = std::collections::HashMap::new();

        // Add project autoloading
        if let Some(ref autoload) = composer.autoload {
            if let Some(ref psr4) = autoload.psr_4 {
                for (namespace, path) in psr4 {
                    let path_str = match path {
                        crate::composer::StringOrArray::Single(p) => p.clone(),
                        crate::composer::StringOrArray::Multiple(paths) => {
                            paths.join(",")
                        }
                    };
                    autoload_mappings.insert(namespace.clone(), path_str);
                }
            }
        }

        // TODO: Phase 2 - Collect autoload mappings from installed packages

        if !autoload_mappings.is_empty() {
            crate::composer::generate_autoloader(vendor_dir, &autoload_mappings)?;
        }

        Ok(())
    }
}
