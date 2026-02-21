//! Project configuration parsed from `rava.hcl`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Parsed representation of `rava.hcl`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectConfig {
    pub project:      ProjectMeta,
    pub dependencies: HashMap<String, String>,
    pub dev_dependencies: HashMap<String, String>,
    pub build:        BuildConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectMeta {
    pub name:    String,
    pub version: String,
    pub java:    String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BuildConfig {
    pub target:   String, // native | jar | jlink | docker
    pub main:     String,
    pub optimize: String, // speed | size | debug
}

impl ProjectConfig {
    /// Load and parse a `rava.hcl` file.
    pub fn from_file(path: &std::path::Path) -> rava_common::error::Result<Self> {
        let _ = path;
        // TODO(phase-1): implement HCL parsing via hcl-rs
        Err(rava_common::error::RavaError::Other("config parsing not yet implemented".into()))
    }
}
