//! `rava init` — initialize a new project.

use anyhow::Result;
use clap::Args;

#[derive(Args)]
pub struct InitArgs {
    /// Project name (defaults to current directory name)
    pub name: Option<String>,

    /// Project template: app | lib | cli
    #[arg(long, default_value = "app")]
    pub template: String,
}

pub async fn init(args: InitArgs) -> Result<()> {
    // TODO(phase-1): scaffold rava.hcl + src/Main.java + test/MainTest.java
    eprintln!("rava init: not yet implemented");
    eprintln!("  name: {:?}, template: {}", args.name, args.template);
    Ok(())
}
