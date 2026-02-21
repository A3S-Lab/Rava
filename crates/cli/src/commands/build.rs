//! `rava build` — AOT-compile to a native binary.

use anyhow::{Context, Result};
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct BuildArgs {
    /// Java source file to compile (omit to use main class from rava.hcl)
    pub file: Option<PathBuf>,

    /// Output target: native | jar | jlink | docker
    #[arg(long, default_value = "native")]
    pub target: String,

    /// Optimization level: speed | size | debug
    #[arg(long, default_value = "speed")]
    pub optimize: String,

    /// Target platform for cross-compilation (e.g. linux-amd64)
    #[arg(long)]
    pub platform: Option<String>,

    /// Output binary path
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

pub async fn build(args: BuildArgs) -> Result<()> {
    let file = args.file.ok_or_else(|| anyhow::anyhow!(
        "no file specified — pass a .java file or run from a project directory"
    ))?;

    let source = std::fs::read_to_string(&file)
        .with_context(|| format!("cannot read {}", file.display()))?;

    // Frontend: .java → RIR
    let compiler = rava_frontend::Compiler::new();
    let mut module = compiler.compile(&source, &file)
        .map_err(|e| anyhow::anyhow!("compile error: {}", e))?;

    // Determine output path
    let output = args.output.unwrap_or_else(|| {
        let stem = file.file_stem().unwrap_or_default().to_string_lossy();
        PathBuf::from(format!("target/{}", stem.to_lowercase()))
    });

    // Ensure output directory exists
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("cannot create output directory {}", parent.display()))?;
    }

    // AOT: optimize + codegen → native binary
    let backend = if let Some(ref platform) = args.platform {
        let triple = parse_platform(platform)?;
        Box::new(rava_codegen_cranelift::CraneliftBackend::for_target(triple))
    } else {
        Box::new(rava_codegen_cranelift::CraneliftBackend::new())
    };
    let aot = rava_aot::AotCompiler::with_default_passes(backend);
    aot.compile(&mut module, &output)
        .map_err(|e| anyhow::anyhow!("build error: {}", e))?;

    eprintln!("  → {}", output.display());
    Ok(())
}

/// Map user-friendly platform names to target triples.
fn parse_platform(platform: &str) -> Result<rava_codegen_cranelift::Triple> {
    let triple_str = match platform {
        "linux-amd64" | "linux-x86_64"   => "x86_64-unknown-linux-gnu",
        "linux-arm64" | "linux-aarch64"  => "aarch64-unknown-linux-gnu",
        "linux-musl"  | "linux-amd64-musl" => "x86_64-unknown-linux-musl",
        "macos-amd64" | "macos-x86_64"  => "x86_64-apple-darwin",
        "macos-arm64" | "macos-aarch64" => "aarch64-apple-darwin",
        "windows-amd64" | "windows-x86_64" => "x86_64-pc-windows-msvc",
        other => other, // allow raw triple like "x86_64-unknown-linux-gnu"
    };
    triple_str.parse::<rava_codegen_cranelift::Triple>()
        .map_err(|e| anyhow::anyhow!("invalid platform '{}': {}", platform, e))
}
