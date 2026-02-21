//! Rava MicroRT — the escape-hatch bytecode runtime.
//!
//! MicroRT handles the ~5% of Java code that AOT cannot statically analyze:
//! reflection, dynamic proxy, and dynamic class loading. It is not a full JVM —
//! it is a lean runtime purpose-built as the AOT escape hatch.
//!
//! # Internal component map (§26.1)
//!
//! ```text
//! MicroRT (~3 MB)
//! ├── interpreter  — bytecode execution engine (Rust match dispatch)
//! ├── loader       — three-tier class loader (Bootstrap → Platform → App)
//! ├── verifier     — StackMapTable verification, type safety
//! └── reflection   — runtime metadata queries, augments AOT metadata table
//! ```
//!
//! # Extension points
//!
//! - [`BytecodeDispatcher`] — swap the dispatch strategy (match vs computed-goto)
//! - [`JitCompiler`]        — hot-path JIT (default: stub; Phase 5: Cranelift JIT)

pub mod interpreter;
pub mod loader;
pub mod reflection;
pub mod verifier;

pub use interpreter::{BytecodeDispatcher, Interpreter};
pub use loader::ClassLoader;
pub use reflection::ReflectionEngine;
pub use verifier::BytecodeVerifier;
