//! `rava fmt` — format Java source files.
//!
//! Simple formatter: consistent indentation, brace placement, trailing newline.

use anyhow::{Context, Result};
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct FmtArgs {
    /// Files to format (defaults to all .java files in src/)
    pub files: Vec<PathBuf>,

    /// Check only — exit non-zero if any file would be reformatted
    #[arg(long)]
    pub check: bool,
}

pub fn fmt(args: FmtArgs) -> Result<()> {
    let files = if args.files.is_empty() {
        find_java_files()?
    } else {
        args.files
    };

    if files.is_empty() {
        eprintln!("no .java files found");
        return Ok(());
    }

    let mut changed = 0;
    let mut unchanged = 0;

    for file in &files {
        let source = std::fs::read_to_string(file)
            .with_context(|| format!("cannot read {}", file.display()))?;

        let formatted = format_java(&source);

        if formatted == source {
            unchanged += 1;
        } else if args.check {
            eprintln!("  would reformat {}", file.display());
            changed += 1;
        } else {
            std::fs::write(file, &formatted)
                .with_context(|| format!("cannot write {}", file.display()))?;
            eprintln!("  formatted {}", file.display());
            changed += 1;
        }
    }

    if args.check && changed > 0 {
        anyhow::bail!("{} file(s) would be reformatted", changed);
    }

    eprintln!("{} changed, {} unchanged", changed, unchanged);
    Ok(())
}

fn find_java_files() -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let dirs = ["src", "."];
    for dir in &dirs {
        let dir_path = PathBuf::from(dir);
        if !dir_path.exists() {
            continue;
        }
        collect_java_files(&dir_path, &mut files)?;
        if !files.is_empty() {
            break;
        } // prefer src/ if it has files
    }
    files.sort();
    Ok(files)
}

fn collect_java_files(dir: &PathBuf, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() && path.file_name().map(|n| n != "target").unwrap_or(true) {
            collect_java_files(&path, files)?;
        } else if path.extension().map(|e| e == "java").unwrap_or(false) {
            files.push(path);
        }
    }
    Ok(())
}

/// Simple Java formatter: normalize indentation and ensure trailing newline.
fn format_java(source: &str) -> String {
    let mut result = String::with_capacity(source.len());
    let mut indent = 0i32;
    let mut prev_blank = false;

    for line in source.lines() {
        let trimmed = line.trim();

        // Skip consecutive blank lines
        if trimmed.is_empty() {
            if !prev_blank && !result.is_empty() {
                result.push('\n');
            }
            prev_blank = true;
            continue;
        }
        prev_blank = false;

        // Decrease indent for closing braces
        if trimmed.starts_with('}') || trimmed.starts_with(')') {
            indent -= 1;
            if indent < 0 {
                indent = 0;
            }
        }

        // Write indented line
        for _ in 0..indent {
            result.push_str("    ");
        }
        result.push_str(trimmed);
        result.push('\n');

        // Increase indent after opening braces
        let opens = trimmed.chars().filter(|&c| c == '{').count() as i32;
        let closes = trimmed.chars().filter(|&c| c == '}').count() as i32;
        indent += opens - closes;
        if trimmed.starts_with('}') {
            // Already decremented above, add back the close we counted
            indent += 1;
        }
        if indent < 0 {
            indent = 0;
        }
    }

    // Ensure trailing newline
    if !result.ends_with('\n') {
        result.push('\n');
    }

    result
}
