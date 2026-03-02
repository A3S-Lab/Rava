//! `rava deps` — inspect project dependencies.

use anyhow::Result;
use clap::{Args, Subcommand};
use rava_pkg::{Lockfile, ProjectConfig, ShortNameRegistry};

#[derive(Args)]
pub struct DepsArgs {
    #[command(subcommand)]
    pub command: DepsCommand,
}

#[derive(Subcommand)]
pub enum DepsCommand {
    /// Print dependency tree
    Tree(TreeArgs),
    /// Generate or verify rava.lock
    Lock(LockArgs),
}

#[derive(Args)]
pub struct TreeArgs {
    /// Show dev_dependencies only
    #[arg(long)]
    pub dev: bool,

    /// Show both dependencies and dev_dependencies
    #[arg(long)]
    pub all: bool,
}

#[derive(Args)]
pub struct LockArgs {
    /// Check that rava.lock is up to date without writing changes
    #[arg(long)]
    pub check: bool,
}

pub fn deps(args: DepsArgs) -> Result<()> {
    match args.command {
        DepsCommand::Tree(tree_args) => tree(tree_args),
        DepsCommand::Lock(lock_args) => lock(lock_args),
    }
}

fn tree(args: TreeArgs) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let hcl_path = cwd.join("rava.hcl");

    if !hcl_path.exists() {
        anyhow::bail!("no rava.hcl found — run `rava init` first");
    }

    let config = ProjectConfig::from_file(&hcl_path).map_err(|e| anyhow::anyhow!("{e}"))?;
    let registry = ShortNameRegistry::builtin();

    if args.all {
        print_tree_section("dependencies", &config.dependencies, &registry);
        print_tree_section("dev_dependencies", &config.dev_dependencies, &registry);
        return Ok(());
    }

    if args.dev {
        print_tree_section("dev_dependencies", &config.dev_dependencies, &registry);
    } else {
        print_tree_section("dependencies", &config.dependencies, &registry);
    }

    Ok(())
}

fn print_tree_section(
    name: &str,
    deps: &std::collections::HashMap<String, String>,
    registry: &ShortNameRegistry,
) {
    println!("{name}");
    if deps.is_empty() {
        println!("  (empty)");
        return;
    }

    let mut items: Vec<_> = deps.iter().collect();
    items.sort_by_key(|(k, _)| k.as_str());
    for (key, version) in items {
        let coord = registry.resolve(key);
        if coord == key {
            println!("  - {key}@{version}");
        } else {
            println!("  - {key}@{version} -> {coord}");
        }
    }
}

fn lock(args: LockArgs) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let hcl_path = cwd.join("rava.hcl");
    let lock_path = cwd.join("rava.lock");

    if !hcl_path.exists() {
        anyhow::bail!("no rava.hcl found — run `rava init` first");
    }

    let config = ProjectConfig::from_file(&hcl_path).map_err(|e| anyhow::anyhow!("{e}"))?;
    let generated = Lockfile::from_project_config(&config).map_err(|e| anyhow::anyhow!("{e}"))?;

    if args.check {
        if !lock_path.exists() {
            anyhow::bail!("rava.lock does not exist — run `rava deps lock` first");
        }
        let current = Lockfile::from_file(&lock_path).map_err(|e| anyhow::anyhow!("{e}"))?;
        if current != generated {
            anyhow::bail!("rava.lock is out of date — run `rava deps lock`");
        }
        println!("rava.lock is up to date");
        return Ok(());
    }

    generated
        .to_file(&lock_path)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("  wrote {}", lock_path.display());
    Ok(())
}
