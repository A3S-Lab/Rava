//! AOT backend: RIR → native binary.
//!
//! # Architecture (§25)
//!
//! Core:
//!   - [`AotCompiler`] — runs the optimization pass chain then calls a codegen backend
//!
//! Pass chain (in order, §25.1):
//!   1. `EscapeAnalysisPass`   — stack vs heap allocation decisions
//!   2. `InliningPass`         — inline small/hot methods
//!   3. `DeadCodeElimPass`     — remove unreachable blocks and dead values
//!   4. `ConstFoldingPass`     — evaluate constant expressions at compile time
//!   5. `MetadataTableGenPass` — generate reflection metadata table (Phase 2)
//!   6. `ProxyPregenPass`      — pre-generate proxy classes (Phase 4)
//!   7. `MicroRtBridgePass`    — generate bridging stubs for MicroRT interop instructions
//!
//! Extension points:
//!   - [`OptPass`]        — a single optimization pass (all 7 above implement this)
//!   - [`CodegenBackend`] — emits native code from optimized RIR (default: Cranelift)

pub mod backend;
pub mod compiler;
pub mod pass;
pub mod passes;

pub use backend::CodegenBackend;
pub use compiler::AotCompiler;
pub use pass::OptPass;
