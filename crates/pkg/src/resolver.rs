//! Maven Central resolver — queries repo1.maven.org for latest versions.

use rava_common::error::{RavaError, Result};

/// Query Maven Central for the latest version of a `groupId:artifactId`.
///
/// Uses repo1.maven.org maven-metadata.xml (no auth, no rate limit).
pub fn latest_version(coordinate: &str) -> Result<String> {
    let (group_id, artifact_id) = parse_coordinate(coordinate)?;
    let group_path = group_id.replace('.', "/");

    let url = format!(
        "https://repo1.maven.org/maven2/{group_path}/{artifact_id}/maven-metadata.xml"
    );

    let resp = reqwest::blocking::get(&url).map_err(|e| {
        RavaError::Package(format!("Maven Central request failed: {e}"))
    })?;

    if !resp.status().is_success() {
        return Err(RavaError::Package(format!(
            "package not found on Maven Central: {coordinate} (HTTP {})", resp.status()
        )));
    }

    let xml = resp.text().map_err(|e| {
        RavaError::Package(format!("failed to read Maven Central response: {e}"))
    })?;

    extract_version(&xml)
        .ok_or_else(|| RavaError::Package(format!("no version found for {coordinate}")))
}

/// Split `"groupId:artifactId"` into parts.
pub fn parse_coordinate(coord: &str) -> Result<(&str, &str)> {
    coord.split_once(':').ok_or_else(|| {
        RavaError::Package(format!(
            "invalid coordinate {:?} — expected groupId:artifactId", coord
        ))
    })
}

/// Extract `<latest>` or `<release>` version from maven-metadata.xml.
fn extract_version(xml: &str) -> Option<String> {
    for tag in &["<latest>", "<release>"] {
        if let Some(start) = xml.find(tag) {
            let after = &xml[start + tag.len()..];
            let end_tag = tag.replace('<', "</");
            if let Some(end) = after.find(end_tag.as_str()) {
                return Some(after[..end].trim().to_string());
            }
        }
    }
    None
}

/// Stub for full transitive dependency resolution (Phase 1+).
pub struct DependencyGraph;
