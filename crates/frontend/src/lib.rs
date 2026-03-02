//! Java frontend: lexer → parser → lowerer → RIR.
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

pub mod ast;
pub mod checker;
pub mod compiler;
pub mod lexer;
pub mod lowerer;
pub mod parser;
pub mod resolver;
pub mod traits;

pub use compiler::Compiler;
pub use traits::{Lowerer as LowererTrait, Parser as ParserTrait, TypeChecker};
