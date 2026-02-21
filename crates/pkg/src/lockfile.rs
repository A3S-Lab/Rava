//! `rava.lock` types — §28.2.

use serde::{Deserialize, Serialize};

/// The full lockfile — guarantees reproducible builds.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Lockfile {
    pub packages: Vec<LockedPackage>,
}

/// A single locked dependency with exact version and content hash.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedPackage {
    pub group_id:     String,
    pub artifact_id:  String,
    /// Exact resolved version (no ranges).
    pub version:      String,
    /// SHA-256 of the downloaded JAR — ensures reproducible builds.
    pub sha256:       String,
    /// Download URL.
    pub url:          String,
    /// Transitive dependency coordinates (`"groupId:artifactId:version"`).
    pub dependencies: Vec<String>,
}

impl Lockfile {
    pub fn new() -> Self {
        Self::default()
    }
}
