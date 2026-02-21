//! Extension point traits for the frontend pipeline.

use rava_common::error::Result;
use rava_rir::Module;

/// Parses Java source text into an untyped AST.
/// Replace this to support different Java versions or dialects.
pub trait Parser: Send + Sync {
    fn parse(&self, source: &str, file: &std::path::Path) -> Result<()>;
}

/// Type-checks and resolves names in the AST.
pub trait TypeChecker: Send + Sync {
    fn check(&self) -> Result<()>;
}

/// Lowers the typed AST to RIR.
pub trait Lowerer: Send + Sync {
    fn lower(&self) -> Result<Module>;
}
