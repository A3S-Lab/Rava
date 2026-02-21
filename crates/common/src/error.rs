//! Unified error type for all Rava crates.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, RavaError>;

#[derive(Debug, Error)]
pub enum RavaError {
    #[error("parse error at {location}: {message}")]
    Parse { location: String, message: String },

    #[error("type error at {location}: {message}")]
    Type { location: String, message: String },

    #[error("codegen error: {0}")]
    Codegen(String),

    #[error("package error: {0}")]
    Package(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}
