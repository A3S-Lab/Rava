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
    Build(commands::build::BuildArgs),
    /// Initialize a new project
    Init(commands::init::InitArgs),
    /// Add a dependency
    Add(commands::add::AddArgs),
    /// Run tests
    Test(commands::test::TestArgs),
    /// Format Java source files
    Fmt(commands::fmt::FmtArgs),
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("rava=info".parse()?),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Run(args)   => commands::run::run(args).await,
        Command::Build(args) => commands::build::build(args).await,
        Command::Init(args)  => commands::init::init(args).await,
        Command::Add(args)   => commands::add::add(args).await,
        Command::Test(args)  => commands::test::test(args).await,
        Command::Fmt(args)   => commands::fmt::fmt(args).await,
    }
}
