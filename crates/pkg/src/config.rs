//! Typed `rava.hcl` config — read via hcl-rs serde, write via template.

use rava_common::error::{RavaError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Parsed `rava.hcl` project configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectConfig {
    #[serde(default)]
    pub project: ProjectMeta,
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
    #[serde(default)]
    pub dev_dependencies: HashMap<String, String>,
    #[serde(default)]
    pub build: BuildConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectMeta {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default = "default_java")]
    pub java: String,
    #[serde(default)]
    pub license: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BuildConfig {
    #[serde(default = "default_target")]
    pub target: String,
    #[serde(default)]
    pub main: String,
    #[serde(default = "default_optimize")]
    pub optimize: String,
}

fn default_java() -> String {
    "21".into()
}
fn default_target() -> String {
    "native".into()
}
fn default_optimize() -> String {
    "speed".into()
}

impl ProjectConfig {
    /// Load and parse a `rava.hcl` file.
    pub fn from_file(path: &Path) -> Result<Self> {
        let src = std::fs::read_to_string(path)
            .map_err(|e| RavaError::Other(format!("cannot read {}: {e}", path.display())))?;
        Self::from_str(&src)
    }

    /// Parse from HCL source string.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(src: &str) -> Result<Self> {
        hcl::from_str(src).map_err(|e| RavaError::Other(format!("rava.hcl parse error: {e}")))
    }

    /// Write a human-readable `rava.hcl` to `path`.
    pub fn to_file(&self, path: &Path) -> Result<()> {
        let content = self.to_hcl_string();
        std::fs::write(path, content)
            .map_err(|e| RavaError::Other(format!("cannot write {}: {e}", path.display())))
    }

    /// Render as a pretty HCL string (hand-templated for readability).
    pub fn to_hcl_string(&self) -> String {
        let mut out = String::new();

        // project block
        out.push_str("project {\n");
        out.push_str(&format!("  name    = {:?}\n", self.project.name));
        out.push_str(&format!("  version = {:?}\n", self.project.version));
        out.push_str(&format!("  java    = {:?}\n", self.project.java));
        if !self.project.license.is_empty() {
            out.push_str(&format!("  license = {:?}\n", self.project.license));
        }
        out.push_str("}\n");

        // dependencies block
        if !self.dependencies.is_empty() {
            out.push('\n');
            out.push_str("dependencies = {\n");
            let mut deps: Vec<_> = self.dependencies.iter().collect();
            deps.sort_by_key(|(k, _)| k.as_str());
            for (k, v) in deps {
                out.push_str(&format!("  {:?} = {:?}\n", k, v));
            }
            out.push_str("}\n");
        }

        // dev_dependencies block
        if !self.dev_dependencies.is_empty() {
            out.push('\n');
            out.push_str("dev_dependencies = {\n");
            let mut deps: Vec<_> = self.dev_dependencies.iter().collect();
            deps.sort_by_key(|(k, _)| k.as_str());
            for (k, v) in deps {
                out.push_str(&format!("  {:?} = {:?}\n", k, v));
            }
            out.push_str("}\n");
        }

        // build block
        out.push('\n');
        out.push_str("build {\n");
        out.push_str(&format!("  target   = {:?}\n", self.build.target));
        if !self.build.main.is_empty() {
            out.push_str(&format!("  main     = {:?}\n", self.build.main));
        }
        out.push_str(&format!("  optimize = {:?}\n", self.build.optimize));
        out.push_str("}\n");

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_empty_project() {
        let cfg = ProjectConfig {
            project: ProjectMeta {
                name: "my-app".into(),
                version: "0.1.0".into(),
                java: "21".into(),
                license: "MIT".into(),
            },
            ..Default::default()
        };
        let hcl = cfg.to_hcl_string();
        let parsed = ProjectConfig::from_str(&hcl).unwrap();
        assert_eq!(parsed.project.name, "my-app");
        assert_eq!(parsed.project.java, "21");
    }

    #[test]
    fn roundtrip_with_dependencies() {
        let mut cfg = ProjectConfig::default();
        cfg.project.name = "test".into();
        cfg.project.version = "1.0.0".into();
        cfg.dependencies.insert("junit".into(), "5.10.1".into());
        cfg.dependencies
            .insert("spring-boot-web".into(), "3.2.0".into());

        let hcl = cfg.to_hcl_string();
        let parsed = ProjectConfig::from_str(&hcl).unwrap();
        assert_eq!(
            parsed.dependencies.get("junit").map(|s| s.as_str()),
            Some("5.10.1")
        );
        assert_eq!(parsed.dependencies.len(), 2);
    }
}
