//! Source location tracking.

use serde::{Deserialize, Serialize};

/// A byte range within a source file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub file: std::path::PathBuf,
    pub start: usize,
    pub end: usize,
    pub line: u32,
    pub column: u32,
}

impl Span {
    pub fn new(
        file: impl Into<std::path::PathBuf>,
        start: usize,
        end: usize,
        line: u32,
        column: u32,
    ) -> Self {
        Self {
            file: file.into(),
            start,
            end,
            line,
            column,
        }
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.file.display(), self.line, self.column)
    }
}
