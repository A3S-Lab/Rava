//! Frontend compiler — orchestrates the parse → typecheck → lower pipeline.

use std::path::Path;
use rava_common::error::{RavaError, Result};
use rava_rir::Module;

/// Compiles Java source files to RIR.
///
/// This is the stable core of the frontend. The actual parse/typecheck/lower
/// implementations are pluggable via the traits in [`crate::traits`].
pub struct Compiler;

impl Compiler {
    pub fn new() -> Self {
        Self
    }

    /// Compile a single Java source file to a RIR module.
    ///
    /// Currently returns a stub — real implementation comes in Phase 1.
    pub fn compile(&self, source: &str, file: &Path) -> Result<Module> {
        // TODO(phase-1): wire up real lexer → parser → type checker → lowerer
        let _ = (source, file);
        Err(RavaError::Other("frontend not yet implemented".into()))
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
        let compiler = Compiler::new();
        let result = compiler.compile("class Main {}", &PathBuf::from("Main.java"));
        assert!(result.is_err());
    }
}
