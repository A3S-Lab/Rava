//! `rava remove` — remove a dependency from the current project.

use anyhow::Result;
use clap::Args;
use rava_pkg::ProjectConfig;

#[derive(Args)]
pub struct RemoveArgs {
    /// Dependency key to remove (alias or coordinate key in rava.hcl)
    pub package: String,

    /// Remove from dev_dependencies only
    #[arg(long)]
    pub dev: bool,
}

pub fn remove(args: RemoveArgs) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let hcl_path = cwd.join("rava.hcl");

    if !hcl_path.exists() {
        anyhow::bail!("no rava.hcl found — run `rava init` first");
    }

    let mut config = ProjectConfig::from_file(&hcl_path).map_err(|e| anyhow::anyhow!("{e}"))?;

    let removed_regular = if args.dev {
        false
    } else {
        config.dependencies.remove(&args.package).is_some()
    };
    let removed_dev = config.dev_dependencies.remove(&args.package).is_some();

    if !removed_regular && !removed_dev {
        anyhow::bail!("dependency '{}' not found", args.package);
    }

    config
        .to_file(&hcl_path)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    if removed_regular {
        println!("  removed dependency: {}", args.package);
    }
    if removed_dev {
        println!("  removed dev dependency: {}", args.package);
    }

    Ok(())
}
