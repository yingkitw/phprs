//! Run command - Run scripts from composer.json

use clap::Args;

#[derive(Args)]
pub struct Run {
    /// Script name to run
    #[arg(required = true)]
    script: String,

    /// Arguments to pass to script
    #[arg(trailing_var_arg = true)]
    args: Vec<String>,
}

impl Run {
    pub async fn execute(&self) -> anyhow::Result<()> {
        log::info!("Run command - Phase 4 feature");
        log::warn!("Not yet implemented");
        Ok(())
    }
}
