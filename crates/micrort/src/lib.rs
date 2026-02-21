//! Rava MicroRT — Phase 1: RIR interpreter for direct execution.
//!
//! In Phase 1, MicroRT executes RIR directly (no bytecode).
//! Phase 3 will add the full bytecode interpreter + class loader.

pub mod builtins;
pub mod interpreter;
pub mod loader;
pub mod lowerer_hash;
pub mod reflection;
pub mod rir_interp;
pub mod verifier;

pub use interpreter::{BytecodeDispatcher, Interpreter};
pub use loader::ClassLoader;
pub use reflection::ReflectionEngine;
pub use rir_interp::RirInterpreter;
pub use verifier::BytecodeVerifier;
