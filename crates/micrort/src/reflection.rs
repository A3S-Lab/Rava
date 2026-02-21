//! Reflection engine — runtime metadata queries, augmenting the AOT metadata table.

use rava_common::error::Result;

/// Queries class/method/field metadata at runtime.
///
/// For AOT-known classes, this queries the compile-time metadata table embedded
/// in the binary (fast path). For dynamically loaded classes, it queries the
/// MicroRT class registry (slow path, cached after first lookup).
pub struct ReflectionEngine;

impl ReflectionEngine {
    pub fn new() -> Self {
        Self
    }

    /// Look up a class by its binary name. Returns the class's bytecode if found.
    pub fn find_class(&self, _binary_name: &str) -> Result<Option<Vec<u8>>> {
        // TODO(phase-2): query AOT metadata table; fall back to MicroRT class registry
        Err(rava_common::error::RavaError::Other("reflection engine not yet implemented".into()))
    }
}

impl Default for ReflectionEngine {
    fn default() -> Self {
        Self::new()
    }
}
