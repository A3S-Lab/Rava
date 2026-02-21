//! `rava fmt` — format Java source files.

use anyhow::Result;
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct FmtArgs {
    /// Files to format (defaults to all .java files in src/)
    pub files: Vec<PathBuf>,

    /// Check only — exit non-zero if any file would be reformatted
    #[arg(long)]
    pub check: bool,
}

pub async fn fmt(args: FmtArgs) -> Result<()> {
    // TODO(phase-1): implement google-java-format style formatter
    eprintln!("rava fmt: not yet implemented");
    eprintln!("  files: {:?}", args.files);
    Ok(())
}
