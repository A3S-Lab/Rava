//! `rava test` — run JUnit tests.

use anyhow::Result;
use clap::Args;

#[derive(Args)]
pub struct TestArgs {
    /// Test name pattern filter
    pub pattern: Option<String>,

    /// Watch mode: re-run on file changes
    #[arg(long)]
    pub watch: bool,
}

pub async fn test(args: TestArgs) -> Result<()> {
    // TODO(phase-1): compile test sources, run JUnit5 via MicroRT
    eprintln!("rava test: not yet implemented");
    eprintln!("  pattern: {:?}", args.pattern);
    Ok(())
}
