//! `rava test` — run test files.
//!
//! Finds all *Test.java files in src/ or test/, compiles and runs them.
//! Tests use simple assert methods: assert(cond), assertEqual(a, b).

use anyhow::{Context, Result};
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct TestArgs {
    /// Test name pattern filter
    pub pattern: Option<String>,

    /// Watch mode: re-run on file changes
    #[arg(long)]
    pub watch: bool,
}

pub fn test(args: TestArgs) -> Result<()> {
    let test_files = find_test_files(args.pattern.as_deref())?;

    if test_files.is_empty() {
        eprintln!("no test files found (looking for *Test.java in src/ and test/)");
        return Ok(());
    }

    let mut passed = 0;
    let mut failed = 0;

    for file in &test_files {
        let name = file.display().to_string();
        let source = std::fs::read_to_string(file)
            .with_context(|| format!("cannot read {}", file.display()))?;

        let compiler = rava_frontend::Compiler::new();
        let module = match compiler.compile(&source, file) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("  ✗ {} — compile error: {}", name, e);
                failed += 1;
                continue;
            }
        };

        let interp = rava_micrort::RirInterpreter::new(module);
        match interp.run_main() {
            Ok(()) => {
                eprintln!("  ✓ {}", name);
                passed += 1;
            }
            Err(e) => {
                eprintln!("  ✗ {} — {}", name, e);
                failed += 1;
            }
        }
    }

    eprintln!();
    eprintln!("{} passed, {} failed", passed, failed);

    if failed > 0 {
        anyhow::bail!("{} test(s) failed", failed);
    }
    Ok(())
}

fn find_test_files(pattern: Option<&str>) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let dirs = ["src", "test", "tests"];

    for dir in &dirs {
        let dir_path = PathBuf::from(dir);
        if !dir_path.exists() { continue; }
        collect_test_files(&dir_path, pattern, &mut files)?;
    }

    // Also check root for *Test.java
    if let Ok(entries) = std::fs::read_dir(".") {
        for entry in entries.flatten() {
            let path = entry.path();
            if is_test_file(&path, pattern) {
                files.push(path);
            }
        }
    }

    files.sort();
    Ok(files)
}

fn collect_test_files(dir: &PathBuf, pattern: Option<&str>, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_test_files(&path, pattern, files)?;
        } else if is_test_file(&path, pattern) {
            files.push(path);
        }
    }
    Ok(())
}

fn is_test_file(path: &PathBuf, pattern: Option<&str>) -> bool {
    let name = match path.file_name().and_then(|n| n.to_str()) {
        Some(n) => n,
        None => return false,
    };
    if !name.ends_with("Test.java") { return false; }
    if let Some(pat) = pattern {
        return name.contains(pat);
    }
    true
}
