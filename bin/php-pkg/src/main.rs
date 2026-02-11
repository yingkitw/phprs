//! PHP-RS Package Manager (php-pkg)
//!
//! A Composer-compatible package manager for PHP-RS

mod commands;
mod composer;
mod config;
mod error;
mod registry;
mod resolver;

use clap::{Parser, Subcommand};
use commands::{Build, Init, Install, Publish, Run, Update};
use env_logger::Env;

#[derive(Parser)]
#[command(name = "php-pkg")]
#[command(version = "0.1.0")]
#[command(about = "Composer-compatible package manager for PHP-RS", long_about = None)]
struct Cli {
    /// Sets the level of verbosity
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new PHP project
    Init(Init),
    /// Install dependencies from composer.json
    Install(Install),
    /// Update dependencies to their latest versions
    Update(Update),
    /// Build PHP project to executable
    Build(Build),
    /// Run scripts defined in composer.json
    Run(Run),
    /// Publish package to registry
    Publish(Publish),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logger
    let env = Env::default()
        .filter_or("PHP_PKG_LOG_LEVEL", "info")
        .write_style_or("PHP_PKG_LOG_STYLE", "always");

    env_logger::try_init_from_env(env)?;

    let cli = Cli::parse();

    // Execute command
    match cli.command {
        Commands::Init(cmd) => cmd.execute().await?,
        Commands::Install(cmd) => cmd.execute().await?,
        Commands::Update(cmd) => cmd.execute().await?,
        Commands::Build(cmd) => cmd.execute().await?,
        Commands::Run(cmd) => cmd.execute().await?,
        Commands::Publish(cmd) => cmd.execute().await?,
    }

    Ok(())
}
