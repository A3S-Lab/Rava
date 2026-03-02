//! Maven Central resolver — queries repo1.maven.org for latest versions and downloads JARs.

use rava_common::error::{RavaError, Result};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Query Maven Central for the latest version of a `groupId:artifactId`.
///
/// Uses repo1.maven.org maven-metadata.xml (no auth, no rate limit).
pub fn latest_version(coordinate: &str) -> Result<String> {
    let (group_id, artifact_id) = parse_coordinate(coordinate)?;
    let group_path = group_id.replace('.', "/");

    let url =
        format!("https://repo1.maven.org/maven2/{group_path}/{artifact_id}/maven-metadata.xml");

    let resp = reqwest::blocking::get(&url)
        .map_err(|e| RavaError::Package(format!("Maven Central request failed: {e}")))?;

    if !resp.status().is_success() {
        return Err(RavaError::Package(format!(
            "package not found on Maven Central: {coordinate} (HTTP {})",
            resp.status()
        )));
    }

    let xml = resp
        .text()
        .map_err(|e| RavaError::Package(format!("failed to read Maven Central response: {e}")))?;

    extract_version(&xml)
        .ok_or_else(|| RavaError::Package(format!("no version found for {coordinate}")))
}

/// Download a JAR file from Maven Central.
///
/// Returns the path to the downloaded JAR in the local cache.
pub fn download_jar(
    group_id: &str,
    artifact_id: &str,
    version: &str,
    cache_dir: &Path,
) -> Result<PathBuf> {
    let group_path = group_id.replace('.', "/");
    let jar_filename = format!("{artifact_id}-{version}.jar");
    let url = format!(
        "https://repo1.maven.org/maven2/{group_path}/{artifact_id}/{version}/{jar_filename}"
    );

    // Create cache directory structure: cache_dir/group_id/artifact_id/version/
    let artifact_cache_dir = cache_dir
        .join(group_id)
        .join(artifact_id)
        .join(version);
    fs::create_dir_all(&artifact_cache_dir)
        .map_err(|e| RavaError::Package(format!("failed to create cache directory: {e}")))?;

    let jar_path = artifact_cache_dir.join(&jar_filename);

    // Skip download if JAR already exists in cache
    if jar_path.exists() {
        return Ok(jar_path);
    }

    // Download JAR
    let resp = reqwest::blocking::get(&url)
        .map_err(|e| RavaError::Package(format!("failed to download JAR: {e}")))?;

    if !resp.status().is_success() {
        return Err(RavaError::Package(format!(
            "JAR not found: {group_id}:{artifact_id}:{version} (HTTP {})",
            resp.status()
        )));
    }

    let bytes = resp
        .bytes()
        .map_err(|e| RavaError::Package(format!("failed to read JAR response: {e}")))?;

    // Write JAR to cache
    let mut file = fs::File::create(&jar_path)
        .map_err(|e| RavaError::Package(format!("failed to create JAR file: {e}")))?;
    file.write_all(&bytes)
        .map_err(|e| RavaError::Package(format!("failed to write JAR file: {e}")))?;

    Ok(jar_path)
}

/// Compute SHA-256 hash of a file.
pub fn compute_sha256(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};

    let bytes = fs::read(path)
        .map_err(|e| RavaError::Package(format!("failed to read file for hashing: {e}")))?;

    let hash = Sha256::digest(&bytes);
    Ok(format!("{:x}", hash))
}

/// Split `"groupId:artifactId"` into parts.
pub fn parse_coordinate(coord: &str) -> Result<(&str, &str)> {
    coord.split_once(':').ok_or_else(|| {
        RavaError::Package(format!(
            "invalid coordinate {:?} — expected groupId:artifactId",
            coord
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
