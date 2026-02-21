//! `rava run` — run a Java source file directly.

use anyhow::Result;
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct RunArgs {
    /// Java source file to run (omit to use main class from rava.hcl)
    pub file: Option<PathBuf>,

    /// Watch for file changes and restart automatically
    #[arg(long)]
    pub watch: bool,

    /// Arguments passed to the Java program
    #[arg(last = true)]
    pub program_args: Vec<String>,
}

pub async fn run(args: RunArgs) -> Result<()> {
    // TODO(phase-1): compile source → RIR → AOT → exec
    eprintln!("rava run: not yet implemented");
    eprintln!("  file: {:?}", args.file);
    Ok(())
}
