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

pub fn run(args: RunArgs) -> Result<()> {
    if args.watch {
        return super::watch::run_watch_loop(
            "java sources",
            || run_once(&args),
            || watch_paths(&args),
        );
    }

    run_once(&args)
}

fn run_once(args: &RunArgs) -> Result<()> {
    let compiler = rava_frontend::Compiler::new();

    let module = if let Some(ref file) = args.file {
        let source = std::fs::read_to_string(file)
            .with_context(|| format!("cannot read {}", file.display()))?;
        compiler
            .compile(&source, file)
            .map_err(|e| anyhow::anyhow!("compile error: {}", e))?
    } else {
        // Project mode
        #[cfg(all(feature = "aot", feature = "pkg"))]
        {
            let _main = super::build::resolve_main_from_hcl_pub()?;
            let files = super::build::collect_source_files()?;
            if files.is_empty() {
                anyhow::bail!("no .java files found in src/");
            }
            compiler
                .compile_project(&files)
                .map_err(|e| anyhow::anyhow!("compile error: {}", e))?
        }
        #[cfg(not(all(feature = "aot", feature = "pkg")))]
        anyhow::bail!("no file specified — pass a .java file")
    };

    let interp = rava_micrort::RirInterpreter::new(module);
    interp
        .run_main()
        .map_err(|e| anyhow::anyhow!("runtime error: {}", e))?;

    Ok(())
}

fn watch_paths(args: &RunArgs) -> Result<Vec<PathBuf>> {
    if let Some(file) = &args.file {
        return Ok(vec![file.clone()]);
    }

    #[cfg(all(feature = "aot", feature = "pkg"))]
    {
        let mut files = super::build::collect_source_files()?;
        files.push(PathBuf::from("rava.hcl"));
        files.sort();
        files.dedup();
        Ok(files)
    }

    #[cfg(not(all(feature = "aot", feature = "pkg")))]
    {
        anyhow::bail!("watch mode requires a file argument");
    }
}
