//! # Event Processor
//!
//! Processes file system events from the watcher and decides which
//! files to track. Handles:
//!
//! - **Ignore patterns**: Parses `.tenetignore` files (like `.gitignore`)
//!   and applies default ignore rules for common directories
//! - **Event filtering**: Skips events for ignored files
//! - **Version creation**: Reads changed files, computes hashes,
//!   stores blobs, and updates metadata

use crate::metadata::{MetadataIndex, VersionType};
use crate::storage;
use crate::utils;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Default patterns that are always ignored, even without a `.tenetignore` file.
///
/// These cover common directories and files that should never be versioned:
/// - Version control directories (`.git/`)
/// - Package/dependency directories (`node_modules/`, `target/`)
/// - Cache directories (`.cache/`)
/// - TENET's own data directory (`.tenet/`)
/// - Log files (`*.log`)
/// - Temporary/swap files
const DEFAULT_IGNORE_PATTERNS: &[&str] = &[
    ".git",
    ".git/",
    "node_modules",
    "node_modules/",
    ".cache",
    ".cache/",
    "target",
    "target/",
    ".tenet",
    ".tenet/",
    "*.log",
    "*.tmp",
    "*.swp",
    "*.swo",
    "*~",
    ".DS_Store",
    "Thumbs.db",
];

/// Holds parsed ignore patterns for efficient path matching.
#[derive(Debug, Clone)]
pub struct IgnoreRules {
    /// List of glob-style patterns to ignore
    patterns: Vec<String>,
}

impl IgnoreRules {
    /// Creates a new `IgnoreRules` by loading the `.tenetignore` file
    /// from the watched directory and merging with default patterns.
    ///
    /// The `.tenetignore` file uses the same syntax as `.gitignore`:
    /// - One pattern per line
    /// - Lines starting with `#` are comments
    /// - Empty lines are ignored
    /// - Patterns support basic glob syntax (`*`, `?`)
    pub fn load(watched_dir: &Path) -> Self {
        let mut patterns: Vec<String> = DEFAULT_IGNORE_PATTERNS
            .iter()
            .map(|s| s.to_string())
            .collect();

        // Try to load .tenetignore from the watched directory
        let ignore_file = watched_dir.join(".tenetignore");
        if let Ok(content) = fs::read_to_string(&ignore_file) {
            for line in content.lines() {
                let line = line.trim();
                // Skip empty lines and comments
                if !line.is_empty() && !line.starts_with('#') {
                    patterns.push(line.to_string());
                }
            }
        }

        Self { patterns }
    }

    /// Checks if a given path should be ignored based on the loaded patterns.
    ///
    /// Matches against:
    /// - Full path components (e.g., `node_modules` matches any path containing it)
    /// - File extensions (e.g., `*.log` matches `server.log`)
    /// - Exact file names (e.g., `.DS_Store`)
    pub fn should_ignore(&self, path: &Path, base_dir: &Path) -> bool {
        // Get relative path for matching
        let rel_path = path.strip_prefix(base_dir).unwrap_or(path);

        let path_str = rel_path.to_string_lossy();
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        for pattern in &self.patterns {
            // Check if pattern matches any path component
            if pattern.starts_with('*') {
                // Wildcard pattern like "*.log"
                let suffix = &pattern[1..];
                if file_name.ends_with(suffix) {
                    return true;
                }
            } else if pattern.ends_with('/') {
                // Directory pattern like "node_modules/"
                let dir_name = &pattern[..pattern.len() - 1];
                // Check if any component of the path matches
                for component in rel_path.components() {
                    if component.as_os_str().to_string_lossy() == dir_name {
                        return true;
                    }
                }
            } else {
                // Exact match against filename or path component
                if file_name == *pattern {
                    return true;
                }
                // Also check against path components
                for component in rel_path.components() {
                    if component.as_os_str().to_string_lossy() == *pattern {
                        return true;
                    }
                }
                // Check if the relative path contains the pattern
                if path_str.contains(pattern.as_str()) {
                    return true;
                }
            }
        }

        false
    }
}

/// Represents a processed file event ready for versioning.
#[derive(Debug)]
pub enum ProcessedEvent {
    /// A file was created or modified — store a new version
    CreateOrModify(PathBuf),
    /// A file was deleted — record deletion marker
    Delete(PathBuf),
}

/// Processes a batch of file paths from debounced watcher events.
///
/// For each event:
/// 1. Check if the path should be ignored
/// 2. Determine event type (create/modify vs delete)
/// 3. For existing files: read content, store blob, update metadata
/// 4. For deleted files: record deletion in metadata
///
/// # Arguments
/// * `paths` - List of file paths that changed
/// * `watched_dir` - The root directory being watched
/// * `tenet_dir` - Path to the `.tenet/` directory
/// * `metadata` - Mutable reference to the metadata index
/// * `ignore_rules` - The ignore rules to apply
///
/// # Returns
/// The number of files that were actually processed (not ignored).
pub fn process_events(
    paths: &[PathBuf],
    watched_dir: &Path,
    tenet_dir: &Path,
    metadata: &mut MetadataIndex,
    ignore_rules: &IgnoreRules,
) -> Result<usize> {
    let mut processed_count = 0;

    for path in paths {
        // Skip directories — we only version files
        if path.is_dir() {
            continue;
        }

        // Apply ignore rules
        if ignore_rules.should_ignore(path, watched_dir) {
            continue;
        }

        // Determine event type based on current file existence
        if path.exists() {
            // File exists → create/modify event
            process_file_change(path, watched_dir, tenet_dir, metadata)?;
            processed_count += 1;
        } else {
            // File doesn't exist → deletion event
            if let Ok(rel_path) = utils::relative_path(path, watched_dir) {
                metadata.record_deletion(&rel_path);
                processed_count += 1;
            }
        }
    }

    // Save metadata after processing all events in the batch
    if processed_count > 0 {
        metadata.save(tenet_dir)?;
    }

    Ok(processed_count)
}

/// Processes a single file change (creation or modification).
///
/// Reads the file, computes its hash, checks for changes,
/// stores the blob if new, and updates the metadata index.
fn process_file_change(
    file_path: &Path,
    watched_dir: &Path,
    tenet_dir: &Path,
    metadata: &mut MetadataIndex,
) -> Result<()> {
    // Read file content
    let content = fs::read(file_path)
        .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

    let hash = utils::hash_content(&content);
    let rel_path = utils::relative_path(file_path, watched_dir)?;
    let size = content.len() as u64;

    // Check if content has actually changed (deduplication at metadata level)
    if let Some(latest) = metadata.get_latest_version(&rel_path) {
        if latest.hash == hash {
            // Content unchanged — skip
            return Ok(());
        }
    }

    // Store the blob (handles storage-level dedup too)
    storage::store_blob(tenet_dir, &content)
        .with_context(|| format!("Failed to store version for: {}", rel_path))?;

    // Record in metadata
    metadata.add_version(&rel_path, &hash, size, VersionType::Snapshot);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_ignore_patterns() {
        let rules = IgnoreRules {
            patterns: DEFAULT_IGNORE_PATTERNS
                .iter()
                .map(|s| s.to_string())
                .collect(),
        };

        let base = Path::new("/project");

        assert!(rules.should_ignore(Path::new("/project/.git/config"), base));
        assert!(rules.should_ignore(Path::new("/project/node_modules/pkg/index.js"), base));
        assert!(rules.should_ignore(Path::new("/project/app.log"), base));
        assert!(!rules.should_ignore(Path::new("/project/src/main.rs"), base));
    }
}
