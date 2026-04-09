//! # Storage Engine
//!
//! Implements a content-addressable blob store for TENET.
//!
//! Files are stored as blobs identified by their SHA-256 hash, enabling:
//! - **Deduplication**: Identical file contents are stored only once
//! - **Integrity**: Content can be verified against its hash
//! - **Efficient lookup**: O(1) access by hash
//!
//! ## Storage Layout
//! ```text
//! .tenet/objects/
//!   ├── a1b2c3d4e5f6...ab.blob    # Each blob named by its SHA-256 hash
//!   └── f0e1d2c3b4a5...cd.blob
//! ```

use crate::utils;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// The subdirectory within `.tenet/` where object blobs are stored.
const OBJECTS_DIR: &str = "objects";

/// Stores file content as a blob in the content-addressable store.
///
/// The blob is named by the SHA-256 hash of its content. If a blob
/// with the same hash already exists, the write is skipped (deduplication).
///
/// # Arguments
/// * `tenet_dir` - Path to the `.tenet/` directory
/// * `content` - The raw file content to store
///
/// # Returns
/// The SHA-256 hash string identifying this blob.
///
/// # Errors
/// Returns an error if the blob cannot be written to disk.
pub fn store_blob(tenet_dir: &Path, content: &[u8]) -> Result<String> {
    let hash = utils::hash_content(content);
    let blob_path = get_blob_path(tenet_dir, &hash);

    // Deduplication: skip if this exact content already exists
    if blob_path.exists() {
        return Ok(hash);
    }

    // Ensure objects directory exists
    let objects_dir = tenet_dir.join(OBJECTS_DIR);
    fs::create_dir_all(&objects_dir)
        .context("Failed to create objects directory")?;

    // Write blob atomically to prevent corruption
    utils::atomic_write(&blob_path, content)
        .with_context(|| format!("Failed to store blob: {}", hash))?;

    Ok(hash)
}

/// Reads a blob from the content-addressable store by its hash.
///
/// # Arguments
/// * `tenet_dir` - Path to the `.tenet/` directory
/// * `hash` - The SHA-256 hash identifying the blob
///
/// # Returns
/// The raw content of the blob as a byte vector.
///
/// # Errors
/// Returns an error if the blob doesn't exist or cannot be read.
pub fn read_blob(tenet_dir: &Path, hash: &str) -> Result<Vec<u8>> {
    let blob_path = get_blob_path(tenet_dir, hash);

    let content = fs::read(&blob_path)
        .with_context(|| format!("Failed to read blob: {} ({})", hash, blob_path.display()))?;

    // Verify integrity: ensure content matches expected hash
    let actual_hash = utils::hash_content(&content);
    if actual_hash != hash {
        anyhow::bail!(
            "Blob integrity check failed!\n  Expected: {}\n  Actual:   {}\n  \
             The stored file may be corrupted.",
            hash,
            actual_hash
        );
    }

    Ok(content)
}

/// Checks whether a blob with the given hash exists in the store.
///
/// # Arguments
/// * `tenet_dir` - Path to the `.tenet/` directory
/// * `hash` - The SHA-256 hash to check
pub fn blob_exists(tenet_dir: &Path, hash: &str) -> bool {
    get_blob_path(tenet_dir, hash).exists()
}

/// Returns the total number of blobs in the store.
///
/// Useful for status reporting and diagnostics.
pub fn blob_count(tenet_dir: &Path) -> usize {
    let objects_dir = tenet_dir.join(OBJECTS_DIR);
    if !objects_dir.exists() {
        return 0;
    }

    fs::read_dir(&objects_dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .map(|ext| ext == "blob")
                        .unwrap_or(false)
                })
                .count()
        })
        .unwrap_or(0)
}

/// Calculates the total size of all blobs in the store (in bytes).
///
/// Useful for status reporting and storage usage monitoring.
pub fn total_storage_size(tenet_dir: &Path) -> u64 {
    let objects_dir = tenet_dir.join(OBJECTS_DIR);
    if !objects_dir.exists() {
        return 0;
    }

    fs::read_dir(&objects_dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter_map(|e| e.metadata().ok())
                .map(|m| m.len())
                .sum()
        })
        .unwrap_or(0)
}

/// Constructs the filesystem path for a blob given its hash.
fn get_blob_path(tenet_dir: &Path, hash: &str) -> PathBuf {
    tenet_dir.join(OBJECTS_DIR).join(format!("{}.blob", hash))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_store_and_read_blob() {
        let dir = tempdir().unwrap();
        let tenet_dir = dir.path().join(".tenet");
        fs::create_dir_all(tenet_dir.join("objects")).unwrap();

        let content = b"Hello, TENET!";
        let hash = store_blob(&tenet_dir, content).unwrap();

        assert!(blob_exists(&tenet_dir, &hash));

        let retrieved = read_blob(&tenet_dir, &hash).unwrap();
        assert_eq!(retrieved, content);
    }

    #[test]
    fn test_deduplication() {
        let dir = tempdir().unwrap();
        let tenet_dir = dir.path().join(".tenet");
        fs::create_dir_all(tenet_dir.join("objects")).unwrap();

        let content = b"duplicate content";
        let hash1 = store_blob(&tenet_dir, content).unwrap();
        let hash2 = store_blob(&tenet_dir, content).unwrap();

        assert_eq!(hash1, hash2, "Same content should produce same hash");
        assert_eq!(blob_count(&tenet_dir), 1, "Should only store one blob");
    }
}
