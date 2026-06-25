//! Frontend compiler — orchestrates the lex → parse → lower pipeline.

use crate::{checker::GenericChecker, lexer::Lexer, lowerer::Lowerer, parser::Parser};
use rava_common::error::Result;
use rava_rir::Module;
use std::path::{Path, PathBuf};

/// Compiles Java source files to RIR.
pub struct Compiler;

impl Compiler {
    pub fn new() -> Self {
        Self
    }

    /// Compile a single Java source file to a RIR module.
    ///
    /// Sources with only top-level statements (no type declaration) are treated as
    /// scripts and auto-wrapped in a synthetic `Main` class so they run directly
    /// (Bun-style scripting, e.g. `rava run script.java`).
    pub fn compile(&self, source: &str, file: &Path) -> Result<Module> {
        let module_name = file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("module");
        let wrapped = wrap_script_if_needed(source);
        let source = wrapped.as_deref().unwrap_or(source);
        let tokens = Lexer::new(source).tokenize()?;
        let ast = Parser::new(tokens).parse_file()?;
        GenericChecker::check_file(&ast)?;
        Lowerer::new(module_name).lower_file(&ast)
    }

    /// Compile multiple Java source files into a single merged RIR module.
    ///
    /// Uses the import resolver to discover and compile transitive dependencies.
    /// Source roots default to the parent directories of the input files + "src/".
    pub fn compile_project(&self, files: &[PathBuf]) -> Result<Module> {
        // Infer source roots from input files
        let mut source_roots = Vec::new();
        let src_dir = PathBuf::from("src");
        if src_dir.exists() {
            source_roots.push(src_dir);
        }
        // Also add parent dirs of input files as roots (for flat layouts)
        for f in files {
            if let Some(parent) = f.parent() {
                let p = parent.to_path_buf();
                if !source_roots.contains(&p) {
                    source_roots.push(p);
                }
            }
        }

        let mut resolver = crate::resolver::ImportResolver::new(source_roots);
        for file in files {
            resolver.compile_file(file, self)?;
        }
        Ok(resolver.into_module())
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Detect "script" sources (only top-level statements, no type declaration) and wrap
/// them in a synthetic `Main` class with a `main` method. Returns `None` when the source
/// already declares a type, in which case it is compiled unchanged.
// ponytail: line-scan heuristic — a `class`/`record`-leading line inside a string/text
// block could false-positive; acceptable for a script runner. Mixed scripts (a type decl
// *and* top-level statements) are not supported; declare a class explicitly for those.
fn wrap_script_if_needed(source: &str) -> Option<String> {
    if declares_top_level_type(source) {
        return None;
    }
    let mut headers = Vec::new();
    let mut body = Vec::new();
    for line in source.lines() {
        let t = line.trim_start();
        if t.starts_with("import ") || t.starts_with("package ") {
            headers.push(line);
        } else {
            body.push(line);
        }
    }
    Some(format!(
        "{headers}\npublic class Main {{\n    public static void main(String[] args) throws Exception {{\n{body}\n    }}\n}}\n",
        headers = headers.join("\n"),
        body = body.join("\n"),
    ))
}

/// True if the source declares a top-level type (class / interface / enum / record /
/// annotation), after stripping leading modifiers.
fn declares_top_level_type(source: &str) -> bool {
    const MODS: &[&str] = &[
        "public ",
        "final ",
        "abstract ",
        "sealed ",
        "non-sealed ",
        "static ",
        "private ",
        "protected ",
        "strictfp ",
    ];
    for line in source.lines() {
        let mut t = line.trim_start();
        while let Some(rest) = MODS.iter().find_map(|m| t.strip_prefix(m)) {
            t = rest.trim_start();
        }
        if t.starts_with("class ")
            || t.starts_with("interface ")
            || t.starts_with("enum ")
            || t.starts_with("record ")
            || t.starts_with("@interface ")
        {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn compile_returns_not_implemented() {
        // kept for compatibility — now it should succeed
        let compiler = Compiler::new();
        let result = compiler.compile("class Main {}", &PathBuf::from("Main.java"));
        assert!(result.is_ok());
    }

    #[test]
    fn compile_hello_world() {
        let src = r#"class Main {
            public static void main(String[] args) {
                System.out.println("Hello, World!");
            }
        }"#;
        let compiler = Compiler::new();
        let module = compiler.compile(src, &PathBuf::from("Main.java")).unwrap();
        assert_eq!(module.functions.len(), 1);
    }
}
