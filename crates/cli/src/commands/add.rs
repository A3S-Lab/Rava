//! `rava add` — add a dependency.

use anyhow::Result;
use clap::Args;

#[derive(Args)]
pub struct AddArgs {
    /// Package to add (e.g. "org.springframework.boot:spring-boot-starter-web")
    pub package: String,

    /// Version constraint (defaults to latest)
    pub version: Option<String>,

    /// Add as a dev dependency
    #[arg(long)]
    pub dev: bool,
}

pub async fn add(args: AddArgs) -> Result<()> {
    // TODO(phase-1): resolve via Maven Central, update rava.hcl + rava.lock
    eprintln!("rava add: not yet implemented");
    eprintln!("  package: {}, version: {:?}", args.package, args.version);
    Ok(())
}
