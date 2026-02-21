//! Cranelift AOT codegen backend.
//!
//! This is the default implementation of [`rava_aot::CodegenBackend`].
//! It translates RIR functions to native machine code via Cranelift.

pub mod backend;
pub mod translator;

pub use backend::CraneliftBackend;
