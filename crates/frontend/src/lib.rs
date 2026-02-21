//! Java frontend: lexer → parser → type checker → semantic analyzer → RIR.
//!
//! # Architecture (Minimal Core + Extension Points)
//!
//! Core (non-replaceable):
//!   - [`Compiler`] — orchestrates the full frontend pipeline
//!
//! Extension points (trait-based, all have defaults):
//!   - [`Parser`]       — Java source → AST
//!   - [`TypeChecker`]  — AST → typed AST
//!   - [`Lowerer`]      — typed AST → RIR

pub mod compiler;
pub mod traits;

pub use compiler::Compiler;
pub use traits::{Lowerer, Parser, TypeChecker};
