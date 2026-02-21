//! Frontend compiler — orchestrates the lex → parse → lower pipeline.

use std::path::Path;
use rava_common::error::Result;
use rava_rir::Module;
use crate::{lexer::Lexer, lowerer::Lowerer, parser::Parser};

/// Compiles Java source files to RIR.
pub struct Compiler;

impl Compiler {
    pub fn new() -> Self { Self }

    /// Compile a single Java source file to a RIR module.
    pub fn compile(&self, source: &str, file: &Path) -> Result<Module> {
        let module_name = file.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("module");
        let tokens = Lexer::new(source).tokenize()?;
        let ast    = Parser::new(tokens).parse_file()?;
        Lowerer::new(module_name).lower_file(&ast)
    }
}

impl Default for Compiler {
    fn default() -> Self { Self::new() }
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
