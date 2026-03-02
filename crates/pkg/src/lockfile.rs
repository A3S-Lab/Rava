//! `rava.lock` types — §28.2.

use crate::{parse_coordinate, ProjectConfig, ShortNameRegistry};
use rava_common::error::{RavaError, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// The full lockfile — guarantees reproducible builds.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Lockfile {
    pub packages: Vec<LockedPackage>,
}

/// A single locked dependency with exact version and content hash.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LockedPackage {
    pub group_id: String,
    pub artifact_id: String,
    /// Exact resolved version (no ranges).
    pub version: String,
    /// SHA-256 of the downloaded JAR — ensures reproducible builds.
    pub sha256: String,
    /// Download URL.
    pub url: String,
    /// Transitive dependency coordinates (`"groupId:artifactId:version"`).
    pub dependencies: Vec<String>,
}

impl Lockfile {
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a lockfile from project config.
    pub fn from_project_config(config: &ProjectConfig) -> Result<Self> {
        let registry = ShortNameRegistry::builtin();
        let mut entries: BTreeMap<(String, String), LockedPackage> = BTreeMap::new();

        for (key, version) in config
            .dependencies
            .iter()
            .chain(config.dev_dependencies.iter())
        {
            let coordinate = registry.resolve(key);
            let (group_id, artifact_id) = parse_coordinate(coordinate)?;
            let url = maven_jar_url(group_id, artifact_id, version);
            let pkg = LockedPackage {
                group_id: group_id.to_string(),
                artifact_id: artifact_id.to_string(),
                version: version.clone(),
                sha256: "unverified".to_string(),
                url,
                dependencies: Vec::new(),
            };
            entries.insert((pkg.group_id.clone(), pkg.artifact_id.clone()), pkg);
        }

        Ok(Self {
            packages: entries.into_values().collect(),
        })
    }

    pub fn from_file(path: &Path) -> Result<Self> {
        let src = std::fs::read_to_string(path)
            .map_err(|e| RavaError::Other(format!("cannot read {}: {e}", path.display())))?;
        hcl::from_str(&src).map_err(|e| RavaError::Other(format!("rava.lock parse error: {e}")))
    }

    pub fn to_file(&self, path: &Path) -> Result<()> {
        let body = hcl::to_string(self)
            .map_err(|e| RavaError::Other(format!("rava.lock serialize error: {e}")))?;
        std::fs::write(path, body)
            .map_err(|e| RavaError::Other(format!("cannot write {}: {e}", path.display())))
    }
}

fn maven_jar_url(group_id: &str, artifact_id: &str, version: &str) -> String {
    let group_path = group_id.replace('.', "/");
    format!(
        "https://repo1.maven.org/maven2/{group_path}/{artifact_id}/{version}/{artifact_id}-{version}.jar"
    )
}
