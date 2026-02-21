//! Shared types used across all Rava crates.
//!
//! - [`error`]      — unified error type and Result alias
//! - [`span`]       — source location (file, line, column)
//! - [`types`]      — Java type system primitives
//! - [`diagnostic`] — structured compiler diagnostics (§23.2)

pub mod diagnostic;
pub mod error;
pub mod span;
pub mod types;
