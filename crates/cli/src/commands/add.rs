//! `rava add` — add a dependency to the current project.

use anyhow::Result;
use clap::Args;
use rava_pkg::{ProjectConfig, ShortNameRegistry, latest_version, parse_coordinate};

#[derive(Args)]
pub struct AddArgs {
    /// Package to add — short name (e.g. "junit") or full coordinate (e.g. "org.junit.jupiter:junit-jupiter")
    pub package: String,

    /// Pin to a specific version (default: fetch latest from Maven Central)
    pub version: Option<String>,

    /// Add as a dev dependency
    #[arg(long)]
    pub dev: bool,
}

pub fn add(args: AddArgs) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let hcl_path = cwd.join("rava.hcl");

    if !hcl_path.exists() {
        anyhow::bail!("no rava.hcl found — run `rava init` first");
    }

    let registry = ShortNameRegistry::builtin();
    let coordinate = registry.resolve(&args.package).to_string();

    parse_coordinate(&coordinate)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let version = match args.version {
        Some(v) => v,
        None => {
            print!("  fetching latest version of {coordinate} ... ");
            let v = latest_version(&coordinate)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            println!("{v}");
            v
        }
    };

    let mut config = ProjectConfig::from_file(&hcl_path)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let display_key = args.package.clone();

    if args.dev {
        config.dev_dependencies.insert(display_key.clone(), version.clone());
    } else {
        config.dependencies.insert(display_key.clone(), version.clone());
    }

    config.to_file(&hcl_path)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let dep_type = if args.dev { "dev dependency" } else { "dependency" };
    println!("  added {dep_type}: {display_key} = \"{version}\"");

    Ok(())
}
