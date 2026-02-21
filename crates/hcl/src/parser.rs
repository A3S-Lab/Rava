//! HCL parser for `rava.hcl`.

use rava_common::error::{RavaError, Result};
use std::path::Path;

/// Parse a `rava.hcl` file into a raw HCL body.
///
/// Returns the parsed HCL body for callers to extract typed config from.
/// TODO(phase-1): return a typed `ProjectConfig` once the schema is stable.
pub fn parse_project_config(path: &Path) -> Result<hcl::Body> {
    let source = std::fs::read_to_string(path).map_err(|e| {
        RavaError::Other(format!("failed to read {}: {e}", path.display()))
    })?;
    hcl::from_str(&source).map_err(|e| {
        RavaError::Other(format!("failed to parse {}: {e}", path.display()))
    })
}
