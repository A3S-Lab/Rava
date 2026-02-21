//! Cranelift AOT codegen backend.
//!
//! This is the default implementation of [`rava_aot::CodegenBackend`].
//! It translates RIR functions to native machine code via Cranelift.
//!
//! Cranelift is chosen as the default because:
//! - Pure Rust, no LLVM dependency
//! - Fast compile times (important for `rava run` developer experience)
//! - Production-quality codegen (used by Wasmtime, rustc_codegen_cranelift)
//!
//! The LLVM backend (`rava-codegen-llvm`) is an optional alternative for
//! release builds where maximum codegen quality matters more than compile speed.

pub mod backend;

pub use backend::CraneliftBackend;
