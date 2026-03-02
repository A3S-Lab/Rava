//! Rava CLI entry point.

mod commands;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name    = "rava",
    version,
    about   = "Java AOT compiler and all-in-one toolchain",
    long_about = None,
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run a Java source file directly
    Run(commands::run::RunArgs),
    /// AOT-compile to a native binary
    #[cfg(feature = "aot")]
    Build(commands::build::BuildArgs),
    /// Initialize a new project
    #[cfg(feature = "pkg")]
    Init(commands::init::InitArgs),
    /// Add a dependency
    #[cfg(feature = "pkg")]
    Add(commands::add::AddArgs),
    /// Remove a dependency
    #[cfg(feature = "pkg")]
    Remove(commands::remove::RemoveArgs),
    /// Update dependency versions
    #[cfg(feature = "pkg")]
    Update(commands::update::UpdateArgs),
    /// Inspect dependency graph
    #[cfg(feature = "pkg")]
    Deps(commands::deps::DepsArgs),
    /// Run tests
    Test(commands::test::TestArgs),
    /// Format Java source files
    Fmt(commands::fmt::FmtArgs),
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env().add_directive("rava=info".parse()?),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Run(args) => commands::run::run(args),
        #[cfg(feature = "aot")]
        Command::Build(args) => commands::build::build(args),
        #[cfg(feature = "pkg")]
        Command::Init(args) => commands::init::init(args),
        #[cfg(feature = "pkg")]
        Command::Add(args) => commands::add::add(args),
        #[cfg(feature = "pkg")]
        Command::Remove(args) => commands::remove::remove(args),
        #[cfg(feature = "pkg")]
        Command::Update(args) => commands::update::update(args),
        #[cfg(feature = "pkg")]
        Command::Deps(args) => commands::deps::deps(args),
        Command::Test(args) => commands::test::test(args),
        Command::Fmt(args) => commands::fmt::fmt(args),
    }
}
