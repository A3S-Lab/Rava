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

pub fn build(args: BuildArgs) -> Result<()> {
    let compiler = rava_frontend::Compiler::new();

    let (mut module, main_file) = if let Some(ref file) = args.file {
        let source = std::fs::read_to_string(file)
            .with_context(|| format!("cannot read {}", file.display()))?;
        let m = compiler.compile(&source, file)
            .map_err(|e| anyhow::anyhow!("compile error: {}", e))?;
        (m, file.clone())
    } else {
        let main_file = resolve_main_from_hcl()?;
        let files = collect_source_files()?;
        if files.is_empty() {
            anyhow::bail!("no .java files found in src/");
        }
        let m = compiler.compile_project(&files)
            .map_err(|e| anyhow::anyhow!("compile error: {}", e))?;
        (m, main_file)
    };

    let output = args.output.unwrap_or_else(|| {
        let stem = main_file.file_stem().unwrap_or_default().to_string_lossy();
        PathBuf::from(format!("target/{}", stem.to_lowercase()))
    });

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("cannot create output directory {}", parent.display()))?;
    }

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

fn resolve_main_from_hcl() -> Result<PathBuf> {
    resolve_main_from_hcl_pub()
}

pub fn resolve_main_from_hcl_pub() -> Result<PathBuf> {
    let hcl_path = PathBuf::from("rava.hcl");
    if !hcl_path.exists() {
        anyhow::bail!("no file specified and no rava.hcl found — pass a .java file or run `rava init`");
    }
    let config = rava_pkg::ProjectConfig::from_file(&hcl_path)
        .map_err(|e| anyhow::anyhow!("cannot read rava.hcl: {e}"))?;
    let main_class = &config.build.main;
    let candidates = [
        PathBuf::from(format!("src/{main_class}.java")),
        PathBuf::from(format!("{main_class}.java")),
    ];
    for c in &candidates {
        if c.exists() { return Ok(c.clone()); }
    }
    anyhow::bail!(
        "main class '{}' not found — expected src/{}.java or {}.java",
        main_class, main_class, main_class
    )
}

fn parse_platform(platform: &str) -> Result<rava_codegen_cranelift::Triple> {
    let triple_str = match platform {
        "linux-amd64" | "linux-x86_64"   => "x86_64-unknown-linux-gnu",
        "linux-arm64" | "linux-aarch64"  => "aarch64-unknown-linux-gnu",
        "linux-musl"  | "linux-amd64-musl" => "x86_64-unknown-linux-musl",
        "macos-amd64" | "macos-x86_64"  => "x86_64-apple-darwin",
        "macos-arm64" | "macos-aarch64" => "aarch64-apple-darwin",
        "windows-amd64" | "windows-x86_64" => "x86_64-pc-windows-msvc",
        other => other,
    };
    triple_str.parse::<rava_codegen_cranelift::Triple>()
        .map_err(|e| anyhow::anyhow!("invalid platform '{}': {}", platform, e))
}

pub fn collect_source_files() -> Result<Vec<PathBuf>> {
    let src_dir = PathBuf::from("src");
    if !src_dir.exists() {
        anyhow::bail!("src/ directory not found");
    }
    let mut files = Vec::new();
    collect_java_recursive(&src_dir, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_java_recursive(dir: &PathBuf, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("cannot read directory {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_java_recursive(&path, out)?;
        } else if path.extension().map(|e| e == "java").unwrap_or(false) {
            out.push(path);
        }
    }
    Ok(())
}
