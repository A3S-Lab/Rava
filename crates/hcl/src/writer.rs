//! HCL writer for generating `rava.hcl` and `rava.lock`.

use rava_common::error::{RavaError, Result};
use std::path::Path;

/// Write an HCL body to a file.
pub fn write_project_config(body: &hcl::Body, path: &Path) -> Result<()> {
    let content = hcl::to_string(body).map_err(|e| {
        RavaError::Other(format!("failed to serialize HCL: {e}"))
    })?;
    std::fs::write(path, content).map_err(|e| {
        RavaError::Other(format!("failed to write {}: {e}", path.display()))
    })
}
