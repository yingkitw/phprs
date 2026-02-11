//! PHP-RS Unified CLI
//!
//! Single binary with three commands:
//! - `phprs run <file>`   — Execute a PHP file
//! - `phprs serve`        — Start the web playground server
//! - `phprs pkg <cmd>`    — Package manager (Composer-compatible)

mod pkg;
mod server;

use clap::{Parser, Subcommand};
use env_logger::Env;

#[derive(Parser)]
#[command(name = "phprs")]
#[command(version = "0.1.0")]
#[command(about = "PHP interpreter, server, and package manager written in Rust")]
struct Cli {
    /// Sets the level of verbosity
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute a PHP file
    Run {
        /// PHP file to execute
        file: String,
    },

    /// Start the web playground server
    Serve {
        /// Port to listen on
        #[arg(short, long, default_value = "3080")]
        port: u16,
    },

    /// Package manager (Composer-compatible)
    Pkg {
        #[command(subcommand)]
        command: pkg::PkgCommands,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let env = Env::default()
        .filter_or("PHPRS_LOG_LEVEL", "info")
        .write_style_or("PHPRS_LOG_STYLE", "always");
    env_logger::try_init_from_env(env)?;

    let cli = Cli::parse();

    match cli.command {
        Commands::Run { file } => cmd_run(&file),
        Commands::Serve { port } => {
            server::start(port);
            Ok(())
        }
        Commands::Pkg { command } => pkg::execute(command).await,
    }
}

/// Execute a PHP file
fn cmd_run(filename: &str) -> anyhow::Result<()> {
    use phprs::engine::compile::compile_file;
    use phprs::engine::vm::{execute_ex, ExecuteData};
    use phprs::php::output::{php_output_end, php_output_start};

    let op_array = compile_file(filename)
        .map_err(|e| anyhow::anyhow!("Compile error: {}", e))?;

    let _ = php_output_start();
    let mut ed = ExecuteData::new();
    let _result = execute_ex(&mut ed, &op_array);
    let output = php_output_end().unwrap_or_default();

    print!("{}", output);
    Ok(())
}
