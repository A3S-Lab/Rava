//! Package registry / resolver trait — §28.2.

use rava_common::error::Result;
use std::path::PathBuf;

/// A Maven dependency coordinate.
#[derive(Debug, Clone)]
pub struct Dependency {
    pub group_id: String,
    pub artifact_id: String,
    /// Version string — may be a range (`"^3.2.0"`) or exact (`"3.2.0"`).
    pub version: String,
}

impl Dependency {
    pub fn new(
        group_id: impl Into<String>,
        artifact_id: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            group_id: group_id.into(),
            artifact_id: artifact_id.into(),
            version: version.into(),
        }
    }

    /// Parse `"groupId:artifactId:version"` or `"groupId:artifactId"` (version = "*").
    pub fn parse(coord: &str) -> Option<Self> {
        let parts: Vec<&str> = coord.splitn(3, ':').collect();
        match parts.as_slice() {
            [g, a, v] => Some(Self::new(*g, *a, *v)),
            [g, a] => Some(Self::new(*g, *a, "*")),
            _ => None,
        }
    }
}

/// A resolved artifact — the local path to the downloaded JAR.
#[derive(Debug, Clone)]
pub struct ResolvedArtifact {
    pub dep: Dependency,
    pub jar_path: PathBuf,
    pub sha256: String,
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

/// Default Maven Central resolver.
pub struct MavenCentralResolver {
    cache_dir: PathBuf,
}

impl MavenCentralResolver {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    /// Create resolver with default cache directory (~/.rava/cache/maven).
    pub fn default() -> Result<Self> {
        let home = dirs::home_dir()
            .ok_or_else(|| rava_common::error::RavaError::Package("no home directory".into()))?;
        let cache_dir = home.join(".rava").join("cache").join("maven");
        Ok(Self::new(cache_dir))
    }
}

impl ClassResolver for MavenCentralResolver {
    fn name(&self) -> &'static str {
        "maven-central"
    }

    fn resolve(&self, dep: &Dependency) -> Result<ResolvedArtifact> {
        use crate::resolver::{compute_sha256, download_jar, latest_version};

        // Resolve version if wildcard
        let version = if dep.version == "*" {
            let coord = format!("{}:{}", dep.group_id, dep.artifact_id);
            latest_version(&coord)?
        } else {
            dep.version.clone()
        };

        // Download JAR
        let jar_path = download_jar(&dep.group_id, &dep.artifact_id, &version, &self.cache_dir)?;

        // Compute SHA-256
        let sha256 = compute_sha256(&jar_path)?;

        Ok(ResolvedArtifact {
            dep: Dependency::new(&dep.group_id, &dep.artifact_id, version),
            jar_path,
            sha256,
        })
    }

    fn transitive_deps(&self, _dep: &Dependency) -> Result<Vec<Dependency>> {
        // TODO: Parse POM file and extract dependencies
        // For now, return empty list (no transitive resolution)
        Ok(Vec::new())
    }
}
