//! # Versioning Engine
//!
//! Implements the core versioning logic for TENET.
//!
//! ## Architecture
//! The engine uses a **strategy pattern** via the `VersionStrategy` trait,
//! allowing different storage approaches:
//!
//! - **`SnapshotStrategy`** (MVP): Stores full file content for every version.
//!   Simple, fast, and reliable — ideal for small-to-medium files.
//!
//! - **`DeltaStrategy`** (Future): Would store only diffs between versions
//!   using the `similar` crate. More space-efficient for large text files.
//!
//! ## Restoration
//! The `restore_file` function reconstructs a file at a given point in time
//! by looking up the appropriate version in metadata and reading the
//! corresponding blob from storage.

use crate::metadata::{MetadataIndex, VersionType};
use crate::storage;
use crate::utils;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::fs;
use std::path::Path;

/// Trait defining the interface for version storage strategies.
///
/// Implementing this trait allows plugging in different storage
/// approaches (snapshot, delta, compressed, etc.) without changing
/// the rest of the system.
pub trait VersionStrategy {
    /// Store a new version of a file.
    ///
    /// # Arguments
    /// * `file_path` - Absolute path to the file
    /// * `watched_dir` - Root watched directory
    /// * `tenet_dir` - Path to `.tenet/`
    /// * `metadata` - Metadata index to update
    fn store_version(
        &self,
        file_path: &Path,
        watched_dir: &Path,
        tenet_dir: &Path,
        metadata: &mut MetadataIndex,
    ) -> Result<()>;

    /// Restore a file to a specific version.
    ///
    /// # Arguments
    /// * `rel_path` - Relative path of the file
    /// * `hash` - Hash of the version to restore
    /// * `watched_dir` - Root watched directory
    /// * `tenet_dir` - Path to `.tenet/`
    fn restore_version(
        &self,
        rel_path: &str,
        hash: &str,
        watched_dir: &Path,
        tenet_dir: &Path,
    ) -> Result<()>;
}

/// Snapshot-based version storage strategy.
///
/// Stores the complete file content for each version.
/// This is the simplest and most reliable approach:
/// - **Pros**: Fast restore (single blob read), simple, no chain dependencies
/// - **Cons**: More storage for large files with small changes
pub struct SnapshotStrategy;

impl VersionStrategy for SnapshotStrategy {
    fn store_version(
        &self,
        file_path: &Path,
        watched_dir: &Path,
        tenet_dir: &Path,
        metadata: &mut MetadataIndex,
    ) -> Result<()> {
        // Read the current file content
        let content = fs::read(file_path)
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

        let hash = utils::hash_content(&content);
        let rel_path = utils::relative_path(file_path, watched_dir)?;
        let size = content.len() as u64;

        // Skip if content hasn't changed since last version
        if let Some(latest) = metadata.get_latest_version(&rel_path) {
            if latest.hash == hash {
                return Ok(());
            }
        }

        // Store blob (content-addressable, auto-deduplicates)
        storage::store_blob(tenet_dir, &content)?;

        // Record the version in metadata
        metadata.add_version(&rel_path, &hash, size, VersionType::Snapshot);

        Ok(())
    }

    fn restore_version(
        &self,
        rel_path: &str,
        hash: &str,
        watched_dir: &Path,
        tenet_dir: &Path,
    ) -> Result<()> {
        // Read the blob from storage
        let content = storage::read_blob(tenet_dir, hash)
            .with_context(|| format!("Failed to read version {} for {}", hash, rel_path))?;

        // Reconstruct the absolute file path
        let file_path = watched_dir.join(rel_path.replace('/', std::path::MAIN_SEPARATOR_STR));

        // Ensure parent directory exists (file may have been deleted along with its dir)
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        // Write the restored content atomically
        utils::atomic_write(&file_path, &content)
            .with_context(|| format!("Failed to restore file: {}", file_path.display()))?;

        Ok(())
    }
}

/// Restores a file to the version closest to the given timestamp.
///
/// This is the main entry point for the `tenet restore` command.
///
/// # Process
/// 1. Find the version at or before the target timestamp
/// 2. Check if it's a deletion marker (warn the user)
/// 3. Read the blob from storage
/// 4. Write the content to the file atomically
///
/// # Arguments
/// * `rel_path` - Relative path of the file to restore
/// * `timestamp` - Target point in time
/// * `watched_dir` - Root watched directory
/// * `tenet_dir` - Path to `.tenet/`
/// * `metadata` - Metadata index for version lookup
///
/// # Returns
/// A summary string describing what was restored.
pub fn restore_file(
    rel_path: &str,
    timestamp: &DateTime<Utc>,
    watched_dir: &Path,
    tenet_dir: &Path,
    metadata: &MetadataIndex,
) -> Result<String> {
    // Look up the version at the requested time
    let version = metadata
        .find_version_at(rel_path, timestamp)
        .with_context(|| {
            format!(
                "No version found for '{}' at or before {}",
                rel_path,
                utils::format_timestamp(timestamp)
            )
        })?;

    // Handle deletion markers
    if version.version_type == VersionType::Deletion {
        anyhow::bail!(
            "File '{}' was deleted at {}. Cannot restore a deletion.\n\
             Tip: Try an earlier timestamp to restore the file before it was deleted.",
            rel_path,
            utils::format_timestamp(&version.timestamp)
        );
    }

    // Use the snapshot strategy to restore
    let strategy = SnapshotStrategy;
    strategy.restore_version(rel_path, &version.hash, watched_dir, tenet_dir)?;

    Ok(format!(
        "Restored '{}' to version from {} (hash: {}..)",
        rel_path,
        utils::format_timestamp(&version.timestamp),
        &version.hash[..12]
    ))
}

/// Creates a snapshot of all files currently in the watched directory.
///
/// This is useful for creating an initial baseline when first starting
/// to watch a directory. Walks the directory tree and creates a version
/// for every file that isn't ignored.
pub fn create_initial_snapshot(
    watched_dir: &Path,
    tenet_dir: &Path,
    metadata: &mut MetadataIndex,
    ignore_rules: &crate::processor::IgnoreRules,
) -> Result<usize> {
    let strategy = SnapshotStrategy;
    let mut count = 0;

    // Walk the directory recursively
    fn walk_dir(
        dir: &Path,
        watched_dir: &Path,
        tenet_dir: &Path,
        metadata: &mut MetadataIndex,
        ignore_rules: &crate::processor::IgnoreRules,
        strategy: &SnapshotStrategy,
        count: &mut usize,
    ) -> Result<()> {
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("⚠️ Skipping directory {}: {}", dir.display(), e);
                return Ok(());
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();

            // Skip ignored paths
            if ignore_rules.should_ignore(&path, watched_dir) {
                continue;
            }

            if path.is_dir() {
                // Recurse into subdirectories (ignore errors to continue)
                let _ = walk_dir(
                    &path,
                    watched_dir,
                    tenet_dir,
                    metadata,
                    ignore_rules,
                    strategy,
                    count,
                );
            } else if path.is_file() {
                // Store version of this file
                match strategy.store_version(&path, watched_dir, tenet_dir, metadata) {
                    Ok(_) => {
                        *count += 1;
                        if *count % 50 == 0 {
                            let _ = metadata.save(tenet_dir);
                            println!("⏳ Snapshotted {} files...", *count);
                        }
                    },
                    Err(e) => eprintln!("⚠️ Failed to track {}: {}", path.display(), e),
                }
            }
        }

        Ok(())
    }

    walk_dir(
        watched_dir,
        watched_dir,
        tenet_dir,
        metadata,
        ignore_rules,
        &strategy,
        &mut count,
    )?;

    // Save metadata after initial snapshot
    metadata.save(tenet_dir)?;

    Ok(count)
}
