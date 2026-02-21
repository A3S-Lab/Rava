//! HCL parsing and generation for `rava.hcl` and `rava.lock`.
//!
//! This crate is the single place that knows about HCL syntax.
//! All other crates that need config parsing depend on this crate,
//! not directly on `hcl-rs`.

pub mod parser;
pub mod writer;

pub use parser::parse_project_config;
pub use writer::write_project_config;
