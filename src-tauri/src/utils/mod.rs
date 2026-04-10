//! # Utilities Module
//!
//! Provides core utility functions used across the TENET system:
//! - SHA-256 content hashing for version identification
//! - Atomic file writes for crash-safety
//! - Timestamp formatting and parsing
//! - `.tenet/` directory structure initialization

use anyhow::{Context, Result};
use chrono::{DateTime, Local, NaiveDateTime, NaiveTime, Utc};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

/// Name of the hidden directory used by TENET to store all versioning data.
pub const TENET_DIR: &str = ".tenet";

/// Computes the SHA-256 hash of the given byte slice.
///
/// Returns a lowercase hex string representation of the hash.
/// This is used as the content-addressable identifier for file versions.
///
/// # Examples
/// ```
/// let hash = hash_content(b"hello world");
/// assert_eq!(hash.len(), 64); // SHA-256 produces 64 hex chars
/// ```
pub fn hash_content(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    format!("{:x}", result)
}

/// Formats a UTC `DateTime` into a human-readable local time string.
///
/// Output format: `YYYY-MM-DD HH:MM:SS`
pub fn format_timestamp(ts: &DateTime<Utc>) -> String {
    let local: DateTime<Local> = ts.with_timezone(&Local);
    local.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Parses a user-provided time string into a UTC `DateTime`.
///
/// Supports multiple formats:
/// - Full datetime: `"2024-01-15 14:30:00"`
/// - Time only (assumes today): `"14:30"` or `"14:30:00"`
/// - Relative time: `"1h"`, `"30m"`, `"2d"` (ago from now)
///
/// # Errors
/// Returns an error if the string cannot be parsed in any supported format.
pub fn parse_timestamp(s: &str) -> Result<DateTime<Utc>> {
    let s = s.trim();

    // Try ISO 8601 / RFC3339 formats first (used by React frontend)
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Utc));
    }

    // Try relative time format: "1h", "30m", "2d"
    if let Some(duration) = parse_relative_time(s) {
        let now = Utc::now();
        return Ok(now - duration);
    }

    // Try time only (today): "14:30:00" or "14:30"
    if let Ok(time) = NaiveTime::parse_from_str(s, "%H:%M:%S")
        .or_else(|_| NaiveTime::parse_from_str(s, "%H:%M"))
        .or_else(|_| NaiveTime::parse_from_str(s, "%I:%M%p"))
        .or_else(|_| NaiveTime::parse_from_str(s, "%I:%M%P"))
    {
        if let Some(local_dt) = Local::now()
            .date_naive()
            .and_time(time)
            .and_local_timezone(Local)
            .single()
        {
            return Ok(local_dt.with_timezone(&Utc));
        }
    }

    // Try full datetime: "2024-01-15 14:30:00"
    if let Ok(naive) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        if let Some(local_dt) = naive.and_local_timezone(Local).single() {
            return Ok(local_dt.with_timezone(&Utc));
        }
    }

    // Try date + time without seconds: "2024-01-15 14:30"
    if let Ok(naive) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M") {
        if let Some(local_dt) = naive.and_local_timezone(Local).single() {
            return Ok(local_dt.with_timezone(&Utc));
        }
    }

    anyhow::bail!(
        "Cannot parse timestamp '{}'. Supported formats:\n  \
         - Full: 2024-01-15 14:30:00\n  \
         - Time: 14:30 or 14:30:00\n  \
         - Relative: 1h, 30m, 2d (ago from now)",
        s
    )
}

/// Parses relative time strings like "1h", "30m", "2d" into a `chrono::Duration`.
fn parse_relative_time(s: &str) -> Option<chrono::Duration> {
    let s = s.trim().to_lowercase();

    if let Some(num_str) = s.strip_suffix('h') {
        let hours: i64 = num_str.parse().ok()?;
        Some(chrono::Duration::hours(hours))
    } else if let Some(num_str) = s.strip_suffix('m') {
        let minutes: i64 = num_str.parse().ok()?;
        Some(chrono::Duration::minutes(minutes))
    } else if let Some(num_str) = s.strip_suffix('d') {
        let days: i64 = num_str.parse().ok()?;
        Some(chrono::Duration::days(days))
    } else if let Some(num_str) = s.strip_suffix('s') {
        let secs: i64 = num_str.parse().ok()?;
        Some(chrono::Duration::seconds(secs))
    } else {
        None
    }
}

/// Performs an atomic write operation.
///
/// Writes data to a temporary file first, then renames it to the target path.
/// This ensures that the target file is never left in a partially-written state,
/// which is critical for crash safety of metadata files.
///
/// # Process
/// 1. Write content to `<target>.tmp`
/// 2. Rename `<target>.tmp` → `<target>` (atomic on most filesystems)
///
/// # Errors
/// Returns an error if the write or rename operation fails.
pub fn atomic_write(path: &Path, data: &[u8]) -> Result<()> {
    let tmp_path = path.with_extension("tmp");

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    // Write to temporary file
    fs::write(&tmp_path, data)
        .with_context(|| format!("Failed to write temp file: {}", tmp_path.display()))?;

    // Atomically rename to target
    fs::rename(&tmp_path, path).with_context(|| {
        format!(
            "Failed to rename {} -> {}",
            tmp_path.display(),
            path.display()
        )
    })?;

    Ok(())
}

/// Ensures the `.tenet/` directory structure exists within the watched directory.
///
/// Creates the following structure if it doesn't exist:
/// ```text
/// <watched_dir>/.tenet/
///   ├── metadata/
///   ├── objects/
///   └── snapshots/
/// ```
///
/// Returns the path to the `.tenet/` directory.
pub fn ensure_tenet_dir(watched_dir: &Path) -> Result<PathBuf> {
    let tenet_dir = watched_dir.join(TENET_DIR);

    // Create subdirectories
    fs::create_dir_all(tenet_dir.join("metadata")).context("Failed to create .tenet/metadata/")?;
    fs::create_dir_all(tenet_dir.join("objects")).context("Failed to create .tenet/objects/")?;
    fs::create_dir_all(tenet_dir.join("snapshots"))
        .context("Failed to create .tenet/snapshots/")?;

    Ok(tenet_dir)
}

/// Finds the `.tenet/` directory by searching from the given path upward.
///
/// This allows commands like `tenet history` to work from subdirectories
/// of the watched directory.
pub fn find_tenet_dir(start_path: &Path) -> Option<PathBuf> {
    let mut current = if start_path.is_file() {
        start_path.parent()?.to_path_buf()
    } else {
        start_path.to_path_buf()
    };

    loop {
        let candidate = current.join(TENET_DIR);
        if candidate.is_dir() {
            return Some(candidate);
        }
        if !current.pop() {
            return None;
        }
    }
}

/// Returns the relative path of a file from the watched directory.
///
/// This is used to store consistent, portable paths in metadata.
pub fn relative_path(file_path: &Path, base_dir: &Path) -> Result<String> {
    let rel = file_path.strip_prefix(base_dir).with_context(|| {
        format!(
            "Path {} is not under base directory {}",
            file_path.display(),
            base_dir.display()
        )
    })?;
    // Normalize to forward slashes for cross-platform consistency
    Ok(rel.to_string_lossy().replace('\\', "/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_content() {
        let hash1 = hash_content(b"hello world");
        let hash2 = hash_content(b"hello world");
        let hash3 = hash_content(b"different content");

        assert_eq!(hash1, hash2, "Same content should produce same hash");
        assert_ne!(
            hash1, hash3,
            "Different content should produce different hash"
        );
        assert_eq!(hash1.len(), 64, "SHA-256 should produce 64 hex chars");
    }

    #[test]
    fn test_parse_relative_time() {
        assert!(parse_relative_time("1h").is_some());
        assert!(parse_relative_time("30m").is_some());
        assert!(parse_relative_time("2d").is_some());
        assert!(parse_relative_time("invalid").is_none());
    }
}
