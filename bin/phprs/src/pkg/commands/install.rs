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

        let composer = super::super::composer::ComposerJson::load(&composer_json_path)?;
        log::info!("Loaded composer.json: {:?}", composer.name);

        // Load configuration
        let config = super::super::config::Config::load(&project_dir)?;

        // Create vendor directory
        let vendor_dir = project_dir.join(config.vendor_dir());
        if !vendor_dir.exists() {
            std::fs::create_dir_all(&vendor_dir)?;
            log::info!("Created vendor directory");
        }

        // Create autoload cache directory
        std::fs::create_dir_all(&config.cache_dir)?;

        // Resolve and install packages (with transitive dependencies)
        if let Some(ref require) = composer.require {
            let client = super::super::registry::PackagistClient::new(Some(config.registry_url.clone()));
            let resolver = super::super::resolver::DependencyResolver::new(client);
            let resolved = resolver.resolve(require, !self.no_dev).await?;

            log::info!("Installing {} package(s)...", resolved.packages.len());

            for package in &resolved.packages {
                log::info!("Installing {} ({})...", package.name, package.version);
                match self.install_package(
                    &package.name,
                    &package.version,
                    &package.metadata,
                    &vendor_dir,
                    &config,
                ).await {
                    Ok(_) => log::info!("Installed {}", package.name),
                    Err(e) => log::error!("Failed to install {}: {}", package.name, e),
                }
            }

            self.generate_autoloader_from_packages(&resolved.packages, &vendor_dir)?;
        }

        // Generate autoloader
        self.generate_autoloader(&composer, &vendor_dir)?;

        log::info!("");
        log::info!("Installation complete!");

        Ok(())
    }

    /// Install a single package
    async fn install_package(
        &self,
        package_name: &str,
        version: &str,
        metadata: &super::super::registry::VersionMetadata,
        vendor_dir: &std::path::Path,
        config: &super::super::config::Config,
    ) -> anyhow::Result<()> {
        // Check if dist is available
        let dist = metadata.dist.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No dist available for {} {}", package_name, version))?;

        // Download and extract package
        let client = super::super::registry::PackagistClient::new(Some(config.registry_url.clone()));
        let package_dir = client.download_package(
            package_name,
            version,
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
                super::super::composer::copy_dir_all(&extracted_dir, &target_dir)?;
            } else {
                // Multiple files/directories at root
                std::fs::create_dir_all(&target_dir)?;
                super::super::composer::copy_dir_all(&package_dir, &target_dir)?;
            }
        }

        Ok(())
    }

    /// Generate PSR-4 autoloader
    fn generate_autoloader(
        &self,
        composer: &super::super::composer::ComposerJson,
        vendor_dir: &std::path::Path,
    ) -> anyhow::Result<()> {
        let mut autoload_mappings: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();

        // Add project autoloading
        if let Some(ref autoload) = composer.autoload {
            if let Some(ref psr4) = autoload.psr_4 {
                for (namespace, path) in psr4 {
                    let paths = match path {
                        super::super::composer::StringOrArray::Single(p) => vec![p.clone()],
                        super::super::composer::StringOrArray::Multiple(paths) => paths.clone(),
                    };
                    autoload_mappings.insert(namespace.clone(), paths);
                }
            }
        }

        if !autoload_mappings.is_empty() {
            super::super::composer::generate_autoloader(vendor_dir, &autoload_mappings)?;
        }

        Ok(())
    }

    fn generate_autoloader_from_packages(
        &self,
        packages: &[super::super::resolver::ResolvedPackage],
        vendor_dir: &std::path::Path,
    ) -> anyhow::Result<()> {
        let mut autoload_mappings: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();

        for package in packages {
            if let Some(autoload) = package.metadata.autoload.as_ref() {
                if let Some(psr4) = autoload.psr_4.as_ref() {
                    for (namespace, path) in psr4 {
                        let paths = match path {
                            super::super::composer::StringOrArray::Single(p) => vec![p.clone()],
                            super::super::composer::StringOrArray::Multiple(paths) => paths.clone(),
                        };
                        let base_paths = paths
                            .into_iter()
                            .map(|p| vendor_dir
                                .join(package.name.replace('/', "/"))
                                .join(p)
                                .to_string_lossy()
                                .to_string())
                            .collect::<Vec<_>>();
                        autoload_mappings
                            .entry(namespace.clone())
                            .or_insert_with(Vec::new)
                            .extend(base_paths);
                    }
                }
            }
        }

        if !autoload_mappings.is_empty() {
            super::super::composer::generate_autoloader(vendor_dir, &autoload_mappings)?;
        }

        Ok(())
    }
}
