//! Init command - Initialize a new PHP project

use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct Init {
    /// Project directory (default: current directory)
    #[arg(short, long)]
    path: Option<PathBuf>,

    /// Package name (e.g., vendor/package)
    #[arg(short, long)]
    name: Option<String>,

    /// Package description
    #[arg(short, long)]
    description: Option<String>,

    /// Package type (library, project, etc.)
    #[arg(long, default_value = "project")]
    type_: String,

    /// License
    #[arg(short, long)]
    license: Option<String>,
}

impl Init {
    pub async fn execute(&self) -> anyhow::Result<()> {
        let project_dir = self.path.as_ref()
            .unwrap_or(&PathBuf::from("."))
            .canonicalize()?;

        log::info!("Initializing PHP project in: {}", project_dir.display());

        // Check if composer.json already exists
        let composer_json_path = project_dir.join("composer.json");
        if composer_json_path.exists() {
            return Err(anyhow::anyhow!(
                "composer.json already exists in {}",
                project_dir.display()
            ));
        }

        // Create composer.json
        let mut composer = super::super::composer::ComposerJson::new(self.name.clone());

        // Set optional fields
        if let Some(ref desc) = self.description {
            composer.description = Some(desc.clone());
        }

        composer.type_ = Some(self.type_.clone());

        if let Some(ref license) = self.license {
            composer.license = Some(super::super::composer::StringOrArray::Single(license.clone()));
        }

        // Add default autoload
        composer.autoload = Some(super::super::composer::Autoload {
            psr_4: Some({
                let mut map = std::collections::HashMap::new();
                if let Some(ref name) = self.name {
                    // Convert vendor/package to Vendor\Package\
                    let namespace = name
                        .split('/')
                        .map(|s| {
                            let mut chars = s.chars();
                            match chars.next() {
                                None => String::new(),
                                first => first.unwrap().to_uppercase().collect::<String>() + chars.as_str(),
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\\");
                    map.insert(
                        format!("{}\\", namespace),
                        super::super::composer::StringOrArray::Single("src/".to_string())
                    );
                } else {
                    map.insert(
                        "App\\".to_string(),
                        super::super::composer::StringOrArray::Single("src/".to_string())
                    );
                }
                map
            }),
            ..Default::default()
        });

        // Save composer.json
        composer.save(&composer_json_path)?;

        log::info!("Created composer.json");

        // Create directory structure
        let src_dir = project_dir.join("src");
        if !src_dir.exists() {
            std::fs::create_dir_all(&src_dir)?;
            log::info!("Created src/ directory");
        }

        // Create .gitignore
        let gitignore_path = project_dir.join(".gitignore");
        if !gitignore_path.exists() {
            let gitignore_content = r#"/vendor/
/node_modules/
/.idea/
/.vscode/
*.log
.DS_Store
"#;
            std::fs::write(&gitignore_path, gitignore_content)?;
            log::info!("Created .gitignore");
        }

        log::info!("");
        log::info!("Project initialized successfully!");
        log::info!("");
        log::info!("Next steps:");
        log::info!("  1. Add dependencies: phprs pkg install");
        log::info!("  2. Build project: phprs pkg build");

        Ok(())
    }
}
