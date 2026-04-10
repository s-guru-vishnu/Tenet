//! # Metadata Manager
//!
//! Manages the version history index for all tracked files.
//!
//! The metadata index is stored as JSON at `.tenet/metadata/index.json`
//! and maintains a complete history of every file version including:
//! - Content hash (SHA-256)
//! - Timestamp of the change
//! - File size at that point
//! - Type of version (Snapshot, Delta, or Deletion marker)
//!
//! ## Thread Safety
//! The metadata index is loaded/saved from disk. For the MVP, operations
//! are serialized through the event processor. Future versions may use
//! file-level locking for concurrent access.

use crate::utils;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Path to the metadata index file within `.tenet/`.
const INDEX_PATH: &str = "metadata/index.json";

/// The type of version stored for a file entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VersionType {
    /// Full file content stored as a blob
    Snapshot,
    /// Only the diff from previous version (future enhancement)
    Delta,
    /// Marks the file as deleted at this point in time
    Deletion,
}

/// Represents a single version of a file at a specific point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileVersion {
    /// SHA-256 hash of the file content (empty string for deletions)
    pub hash: String,
    /// When this version was recorded
    pub timestamp: DateTime<Utc>,
    /// File size in bytes (0 for deletions)
    pub size: u64,
    /// How this version is stored
    pub version_type: VersionType,
}

/// Tracks the complete version history of a single file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    /// Relative path from the watched directory (uses forward slashes)
    pub path: String,
    /// Ordered list of versions (oldest first)
    pub versions: Vec<FileVersion>,
}

/// The root metadata index containing all tracked files.
///
/// This is serialized to/from `.tenet/metadata/index.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataIndex {
    /// The absolute path of the directory being watched
    pub watched_dir: String,
    /// Map from relative file path → file entry with version history
    pub files: HashMap<String, FileEntry>,
    /// When this index was first created
    pub created_at: DateTime<Utc>,
    /// When this index was last modified
    pub last_updated: DateTime<Utc>,
}

impl MetadataIndex {
    /// Creates a new, empty metadata index for the given watched directory.
    pub fn new(watched_dir: &str) -> Self {
        let now = Utc::now();
        Self {
            watched_dir: watched_dir.to_string(),
            files: HashMap::new(),
            created_at: now,
            last_updated: now,
        }
    }

    /// Loads the metadata index from disk.
    ///
    /// If the index file doesn't exist, creates a new empty index.
    ///
    /// # Arguments
    /// * `tenet_dir` - Path to the `.tenet/` directory
    /// * `watched_dir` - Path to the watched directory (used if creating new index)
    pub fn load(tenet_dir: &Path, watched_dir: &str) -> Result<Self> {
        let index_path = tenet_dir.join(INDEX_PATH);

        if !index_path.exists() {
            return Ok(Self::new(watched_dir));
        }

        let data = std::fs::read_to_string(&index_path)
            .with_context(|| format!("Failed to read index: {}", index_path.display()))?;

        let index: MetadataIndex = serde_json::from_str(&data)
            .with_context(|| "Failed to parse metadata index (file may be corrupted)")?;

        Ok(index)
    }

    /// Saves the metadata index to disk atomically.
    ///
    /// Uses atomic write (write to temp + rename) to prevent corruption
    /// in case of crashes or power loss.
    pub fn save(&mut self, tenet_dir: &Path) -> Result<()> {
        self.last_updated = Utc::now();

        let index_path = tenet_dir.join(INDEX_PATH);
        let data =
            serde_json::to_string_pretty(self).context("Failed to serialize metadata index")?;

        utils::atomic_write(&index_path, data.as_bytes())
            .context("Failed to save metadata index")?;

        Ok(())
    }

    /// Records a new file version in the metadata index.
    ///
    /// If the file hasn't been tracked before, a new entry is created.
    /// The version is appended to the end of the version list.
    ///
    /// # Arguments
    /// * `rel_path` - Relative path of the file from the watched directory
    /// * `hash` - SHA-256 hash of the file content
    /// * `size` - File size in bytes
    /// * `version_type` - How this version is stored (Snapshot/Delta/Deletion)
    pub fn add_version(
        &mut self,
        rel_path: &str,
        hash: &str,
        size: u64,
        version_type: VersionType,
    ) {
        let version = FileVersion {
            hash: hash.to_string(),
            timestamp: Utc::now(),
            size,
            version_type,
        };

        let entry = self
            .files
            .entry(rel_path.to_string())
            .or_insert_with(|| FileEntry {
                path: rel_path.to_string(),
                versions: Vec::new(),
            });

        entry.versions.push(version);
    }

    /// Records a file deletion in the metadata.
    ///
    /// Adds a special "Deletion" version marker so the history
    /// shows when the file was deleted.
    pub fn record_deletion(&mut self, rel_path: &str) {
        self.add_version(rel_path, "", 0, VersionType::Deletion);
    }

    /// Retrieves the version history for a specific file.
    ///
    /// Returns `None` if the file has never been tracked.
    pub fn get_history(&self, rel_path: &str) -> Option<&FileEntry> {
        self.files.get(rel_path)
    }

    /// Gets the latest (most recent) version of a file.
    ///
    /// Returns `None` if the file has no versions recorded.
    pub fn get_latest_version(&self, rel_path: &str) -> Option<&FileVersion> {
        self.files
            .get(rel_path)
            .and_then(|entry| entry.versions.last())
    }

    /// Finds the version of a file closest to (at or before) the given timestamp.
    ///
    /// This is the core lookup for the `tenet restore` command.
    /// It finds the most recent version that was recorded at or before
    /// the specified time.
    ///
    /// # Returns
    /// - `Some(&FileVersion)` if a version exists at or before the timestamp
    /// - `None` if no version exists before that time
    pub fn find_version_at(
        &self,
        rel_path: &str,
        timestamp: &DateTime<Utc>,
    ) -> Option<&FileVersion> {
        self.files.get(rel_path).and_then(|entry| {
            // Find the last version whose timestamp is <= the target
            entry
                .versions
                .iter()
                .rev()
                .find(|v| v.timestamp <= *timestamp)
        })
    }

    /// Returns the total number of tracked files.
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Returns the total number of versions across all files.
    pub fn total_versions(&self) -> usize {
        self.files.values().map(|e| e.versions.len()).sum()
    }

    /// Returns all tracked file paths sorted alphabetically.
    pub fn tracked_files(&self) -> Vec<&str> {
        let mut paths: Vec<&str> = self.files.keys().map(|s| s.as_str()).collect();
        paths.sort();
        paths
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_get_version() {
        let mut index = MetadataIndex::new("/test/dir");

        index.add_version("file.txt", "abc123", 100, VersionType::Snapshot);
        index.add_version("file.txt", "def456", 150, VersionType::Snapshot);

        let entry = index.get_history("file.txt").unwrap();
        assert_eq!(entry.versions.len(), 2);

        let latest = index.get_latest_version("file.txt").unwrap();
        assert_eq!(latest.hash, "def456");
    }

    #[test]
    fn test_file_count() {
        let mut index = MetadataIndex::new("/test/dir");

        index.add_version("a.txt", "hash1", 10, VersionType::Snapshot);
        index.add_version("b.txt", "hash2", 20, VersionType::Snapshot);
        index.add_version("a.txt", "hash3", 30, VersionType::Snapshot);

        assert_eq!(index.file_count(), 2);
        assert_eq!(index.total_versions(), 3);
    }
}
