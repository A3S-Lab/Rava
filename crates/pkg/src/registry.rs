//! Package registry / resolver trait — §28.2.

use rava_common::error::Result;
use std::path::PathBuf;

/// A Maven dependency coordinate.
#[derive(Debug, Clone)]
pub struct Dependency {
    pub group_id:    String,
    pub artifact_id: String,
    /// Version string — may be a range (`"^3.2.0"`) or exact (`"3.2.0"`).
    pub version:     String,
}

impl Dependency {
    pub fn new(group_id: impl Into<String>, artifact_id: impl Into<String>, version: impl Into<String>) -> Self {
        Self { group_id: group_id.into(), artifact_id: artifact_id.into(), version: version.into() }
    }

    /// Parse `"groupId:artifactId:version"` or `"groupId:artifactId"` (version = "*").
    pub fn parse(coord: &str) -> Option<Self> {
        let parts: Vec<&str> = coord.splitn(3, ':').collect();
        match parts.as_slice() {
            [g, a, v] => Some(Self::new(*g, *a, *v)),
            [g, a]    => Some(Self::new(*g, *a, "*")),
            _         => None,
        }
    }
}

/// A resolved artifact — the local path to the downloaded JAR.
#[derive(Debug, Clone)]
pub struct ResolvedArtifact {
    pub dep:      Dependency,
    pub jar_path: PathBuf,
    pub sha256:   String,
}

/// Extension point: resolves a dependency coordinate to a local JAR.
///
/// Default: `MavenCentralResolver` (downloads from `repo1.maven.org`).
pub trait ClassResolver: Send + Sync {
    fn name(&self) -> &'static str;
    /// Resolve a dependency → download/locate the JAR file.
    fn resolve(&self, dep: &Dependency) -> Result<ResolvedArtifact>;
    /// List the direct transitive dependencies declared in the POM.
    fn transitive_deps(&self, dep: &Dependency) -> Result<Vec<Dependency>>;
}
