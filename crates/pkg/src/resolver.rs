//! Dependency graph and resolution — §28.1.

use rava_common::error::{RavaError, Result};
use std::collections::HashMap;
use crate::registry::ClassResolver;
use crate::lockfile::Lockfile;

/// Resolves a set of direct dependencies to a complete, conflict-free lock map.
///
/// Conflict resolution rule: for the same `groupId:artifactId`, pick the
/// highest compatible version (Maven "nearest wins" is intentionally NOT used —
/// it causes subtle version downgrades).
#[allow(dead_code)] // used in Phase 1 resolver implementation
pub struct DependencyGraph {
    resolver: Box<dyn ClassResolver>,
}

impl DependencyGraph {
    pub fn new(resolver: Box<dyn ClassResolver>) -> Self {
        Self { resolver }
    }

    /// Resolve all direct deps (and their transitive deps) into a `Lockfile`.
    pub fn resolve(&self, direct_deps: &HashMap<String, String>) -> Result<Lockfile> {
        let _ = direct_deps;
        // TODO(phase-1): implement BFS transitive resolution + conflict resolution
        Err(RavaError::Package("dependency resolver not yet implemented".into()))
    }
}
