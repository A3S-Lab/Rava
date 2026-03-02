//! `rava update` — refresh dependency versions from Maven Central.

use anyhow::Result;
use clap::Args;
use rava_pkg::{latest_version, parse_coordinate, ProjectConfig, ShortNameRegistry};
use std::collections::HashMap;

#[derive(Args)]
pub struct UpdateArgs {
    /// Dependency key to update (omit to update all keys in selected section)
    pub package: Option<String>,

    /// Update dev_dependencies instead of dependencies
    #[arg(long)]
    pub dev: bool,
}

pub fn update(args: UpdateArgs) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let hcl_path = cwd.join("rava.hcl");

    if !hcl_path.exists() {
        anyhow::bail!("no rava.hcl found — run `rava init` first");
    }

    let mut config = ProjectConfig::from_file(&hcl_path).map_err(|e| anyhow::anyhow!("{e}"))?;
    let registry = ShortNameRegistry::builtin();

    if args.dev {
        update_section(
            &mut config.dev_dependencies,
            args.package.as_deref(),
            &registry,
            true,
        )?;
    } else {
        update_section(
            &mut config.dependencies,
            args.package.as_deref(),
            &registry,
            false,
        )?;
    }

    config
        .to_file(&hcl_path)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

fn update_section(
    deps: &mut HashMap<String, String>,
    package: Option<&str>,
    registry: &ShortNameRegistry,
    dev: bool,
) -> Result<()> {
    let dep_label = if dev { "dev dependency" } else { "dependency" };

    if deps.is_empty() {
        anyhow::bail!(
            "no {} entries found",
            if dev {
                "dev_dependencies"
            } else {
                "dependencies"
            }
        );
    }

    match package {
        Some(key) => {
            let old = deps
                .get(key)
                .ok_or_else(|| anyhow::anyhow!("{} '{}' not found", dep_label, key))?
                .clone();
            let latest = resolve_latest_version(key, registry)?;
            deps.insert(key.to_string(), latest.clone());
            println!("  updated {dep_label}: {key} {old} -> {latest}");
        }
        None => {
            let mut keys: Vec<String> = deps.keys().cloned().collect();
            keys.sort();
            for key in keys {
                let old = deps.get(&key).cloned().unwrap_or_default();
                let latest = resolve_latest_version(&key, registry)?;
                deps.insert(key.clone(), latest.clone());
                println!("  updated {dep_label}: {key} {old} -> {latest}");
            }
        }
    }

    Ok(())
}

fn resolve_latest_version(key: &str, registry: &ShortNameRegistry) -> Result<String> {
    let coordinate = registry.resolve(key).to_string();
    parse_coordinate(&coordinate).map_err(|e| anyhow::anyhow!("{e}"))?;
    latest_version(&coordinate).map_err(|e| anyhow::anyhow!("{e}"))
}
