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
    pub fn compile(&self, source: &str, file: &Path) -> Result<Module> {
        let module_name = file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("module");
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
