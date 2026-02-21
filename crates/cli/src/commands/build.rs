//! `rava build` — AOT-compile to a native binary.

use anyhow::Result;
use clap::Args;

#[derive(Args)]
pub struct BuildArgs {
    /// Output target: native | jar | jlink | docker
    #[arg(long, default_value = "native")]
    pub target: String,

    /// Optimization level: speed | size | debug
    #[arg(long, default_value = "speed")]
    pub optimize: String,

    /// Target platform for cross-compilation (e.g. linux-amd64)
    #[arg(long)]
    pub platform: Option<String>,
}

pub async fn build(args: BuildArgs) -> Result<()> {
    // TODO(phase-1): frontend → RIR → opt passes → codegen backend → link
    eprintln!("rava build: not yet implemented");
    eprintln!("  target: {}, optimize: {}", args.target, args.optimize);
    Ok(())
}
