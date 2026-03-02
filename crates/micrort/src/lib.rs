//! Rava MicroRT — Phase 3: RIR interpreter + JVM bytecode interpreter.
//!
//! Phase 1/2: executes RIR directly (`RirInterpreter`).
//! Phase 3: JVM bytecode interpreter (`Interpreter`) + structural verifier
//!          (`BytecodeVerifier`) + reflection engine (`ReflectionEngine`).

pub mod builtins;
pub mod interpreter;
pub mod loader;
pub mod lowerer_hash;
pub mod reflection;
pub mod rir_interp;
pub mod verifier;

pub use interpreter::{BytecodeDispatcher, Interpreter, MatchDispatcher};
pub use loader::ClassLoader;
pub use reflection::ReflectionEngine;
pub use rir_interp::RirInterpreter;
pub use verifier::BytecodeVerifier;
