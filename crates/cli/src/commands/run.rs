//! `rava run` — run a Java source file directly.

use anyhow::{Context, Result};
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct RunArgs {
    /// Java source file to run (omit to use main class from rava.hcl)
    pub file: Option<PathBuf>,

    /// Watch for file changes and restart automatically
    #[arg(long)]
    pub watch: bool,

    /// Arguments passed to the Java program
    #[arg(last = true)]
    pub program_args: Vec<String>,
}

pub async fn run(args: RunArgs) -> Result<()> {
    let file = args.file.ok_or_else(|| anyhow::anyhow!(
        "no file specified — pass a .java file or run from a project directory"
    ))?;

    let source = std::fs::read_to_string(&file)
        .with_context(|| format!("cannot read {}", file.display()))?;

    let compiler = rava_frontend::Compiler::new();
    let module = compiler.compile(&source, &file)
        .map_err(|e| anyhow::anyhow!("compile error: {}", e))?;

    let interp = rava_micrort::RirInterpreter::new(module);
    interp.run_main()
        .map_err(|e| anyhow::anyhow!("runtime error: {}", e))?;

    Ok(())
}
