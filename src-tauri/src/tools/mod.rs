//! # TENET Agent Tools
//!
//! Implements the four tool-callable functions exposed to the rig.rs AI agent.
//! Each tool wraps a TENET core operation so the LLM can invoke them via
//! structured function-calling.
//!
//! ## Tools
//! - [`GetHistoryTool`]  — version timeline for a file
//! - [`RestoreVersionTool`] — restore a file to a past state
//! - [`ListFilesTool`]   — enumerate all tracked files
//! - [`DiffVersionsTool`] — compare two stored versions

use crate::metadata::MetadataIndex;
use crate::utils;
use crate::versioning;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;

// ─── Shared error type ────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
#[error("TENET tool error: {0}")]
pub struct TenetToolError(pub String);

// ─── GetHistory ───────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct GetHistoryArgs {
    pub file: String,
}

pub struct GetHistoryTool {
    pub watched_dir: PathBuf,
    pub tenet_dir: PathBuf,
}

impl Tool for GetHistoryTool {
    const NAME: &'static str = "get_history";
    type Error = TenetToolError;
    type Args = GetHistoryArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Get the complete version history of a tracked file. \
                Returns a list of all saved versions with timestamps, sizes, and hashes."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "file": {
                        "type": "string",
                        "description": "Relative path of the file from the watched directory \
                                        (e.g. 'src/main.rs' or 'README.md')"
                    }
                },
                "required": ["file"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let watched_dir_str = self.watched_dir.to_string_lossy().to_string();
        let metadata = MetadataIndex::load(&self.tenet_dir, &watched_dir_str)
            .map_err(|e| TenetToolError(e.to_string()))?;

        match metadata.get_history(&args.file) {
            None => Ok(format!(
                "No version history found for '{}'. \
                 Make sure the file path is relative to the watched directory.",
                args.file
            )),
            Some(entry) => {
                let mut out = format!(
                    "Version history for '{}' ({} version{}):\n",
                    entry.path,
                    entry.versions.len(),
                    if entry.versions.len() == 1 { "" } else { "s" }
                );
                for (i, v) in entry.versions.iter().enumerate() {
                    let vnum = i + 1;
                    let hash_short = if v.hash.len() >= 8 { &v.hash[..8] } else { &v.hash };
                    out.push_str(&format!(
                        "  v{vnum}: {} | {} bytes | hash:{} | {}\n",
                        v.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
                        v.size,
                        hash_short,
                        v.version_type,
                    ));
                }
                Ok(out)
            }
        }
    }
}

// Helper so VersionType serialises as a string in tool output
impl std::fmt::Display for crate::metadata::VersionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            crate::metadata::VersionType::Snapshot => write!(f, "Snapshot"),
            crate::metadata::VersionType::Delta => write!(f, "Delta"),
            crate::metadata::VersionType::Deletion => write!(f, "Deletion"),
        }
    }
}

// ─── RestoreVersion ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RestoreVersionArgs {
    pub file: String,
    pub time: String,
}

pub struct RestoreVersionTool {
    pub watched_dir: PathBuf,
    pub tenet_dir: PathBuf,
}

impl Tool for RestoreVersionTool {
    const NAME: &'static str = "restore_version";
    type Error = TenetToolError;
    type Args = RestoreVersionArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Restore a file to its state at (or just before) a given point in time. \
                Supports relative times ('1h', '2d', '30m') and ISO timestamps."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "file": {
                        "type": "string",
                        "description": "Relative file path (e.g. 'src/main.rs')"
                    },
                    "time": {
                        "type": "string",
                        "description": "When to restore to. Use relative format: '1h' (1 hour ago), \
                                        '1d' (1 day ago), '30m' (30 minutes ago), \
                                        or an ISO 8601 timestamp."
                    }
                },
                "required": ["file", "time"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let watched_dir_str = self.watched_dir.to_string_lossy().to_string();
        let metadata = MetadataIndex::load(&self.tenet_dir, &watched_dir_str)
            .map_err(|e| TenetToolError(e.to_string()))?;

        // Check if the time argument is a version number like "v1", "v2", "1", "2", etc.
        let version_str = args.time.trim().to_lowercase();
        let version_num = version_str
            .strip_prefix('v')
            .unwrap_or(&version_str);

        if let Ok(idx) = version_num.parse::<usize>() {
            // Restore by version index
            let entry = metadata
                .get_history(&args.file)
                .ok_or_else(|| TenetToolError(format!(
                    "No version history found for '{}'. Make sure the file path is relative to the watched directory.",
                    args.file
                )))?;

            if idx == 0 || idx > entry.versions.len() {
                return Err(TenetToolError(format!(
                    "Version {} out of range. '{}' has {} version(s) (v1–v{}).",
                    idx, args.file, entry.versions.len(), entry.versions.len()
                )));
            }

            let version = &entry.versions[idx - 1];

            if version.version_type == crate::metadata::VersionType::Deletion {
                return Err(TenetToolError(format!(
                    "Version {} of '{}' is a deletion marker — cannot restore.",
                    idx, args.file
                )));
            }

            // Restore using the snapshot strategy directly with the hash
            let strategy = versioning::SnapshotStrategy;
            use crate::versioning::VersionStrategy;
            strategy
                .restore_version(&args.file, &version.hash, &self.watched_dir, &self.tenet_dir)
                .map_err(|e| TenetToolError(e.to_string()))?;

            return Ok(format!("Restored '{}' to v{}", args.file, idx));
        }

        // Fall back to timestamp-based restore
        let timestamp = utils::parse_timestamp(&args.time)
            .map_err(|e| TenetToolError(e.to_string()))?;

        let result = versioning::restore_file(
            &args.file,
            &timestamp,
            &self.watched_dir,
            &self.tenet_dir,
            &metadata,
        )
        .map_err(|e| TenetToolError(e.to_string()))?;

        Ok(result)
    }
}

// ─── ListFiles ────────────────────────────────────────────────────────────────

#[derive(Deserialize, Serialize)]
pub struct ListFilesArgs {}

pub struct ListFilesTool {
    pub watched_dir: PathBuf,
    pub tenet_dir: PathBuf,
}

impl Tool for ListFilesTool {
    const NAME: &'static str = "list_files";
    type Error = TenetToolError;
    type Args = ListFilesArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "List all files currently being tracked by TENET in the watched directory."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {}
            }),
        }
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        let watched_dir_str = self.watched_dir.to_string_lossy().to_string();
        let metadata = MetadataIndex::load(&self.tenet_dir, &watched_dir_str)
            .map_err(|e| TenetToolError(e.to_string()))?;

        let files = metadata.tracked_files();
        if files.is_empty() {
            return Ok("No files are currently tracked. Start watching a directory first.".to_string());
        }

        let body = files
            .iter()
            .map(|f| format!("  - {f}"))
            .collect::<Vec<_>>()
            .join("\n");

        Ok(format!(
            "Tracked files ({} total):\n{body}",
            files.len()
        ))
    }
}

// ─── DiffVersions ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct DiffVersionsArgs {
    pub file: String,
    pub v1: String,
    pub v2: String,
}

pub struct DiffVersionsTool {
    pub watched_dir: PathBuf,
    pub tenet_dir: PathBuf,
}

impl Tool for DiffVersionsTool {
    const NAME: &'static str = "diff_versions";
    type Error = TenetToolError;
    type Args = DiffVersionsArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Compare two stored versions of a file and show what changed. \
                v1 and v2 are version numbers starting at 1, or 'latest' for the newest version."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "file": {
                        "type": "string",
                        "description": "Relative file path (e.g. 'src/main.rs')"
                    },
                    "v1": {
                        "type": "string",
                        "description": "First version to compare (e.g. '1') or 'latest'"
                    },
                    "v2": {
                        "type": "string",
                        "description": "Second version to compare (e.g. '2') or 'latest'"
                    }
                },
                "required": ["file", "v1", "v2"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let watched_dir_str = self.watched_dir.to_string_lossy().to_string();
        let metadata = MetadataIndex::load(&self.tenet_dir, &watched_dir_str)
            .map_err(|e| TenetToolError(e.to_string()))?;

        let entry = metadata
            .get_history(&args.file)
            .ok_or_else(|| TenetToolError(format!("No history for '{}'", args.file)))?;

        let resolve = |v: &str| -> Result<&crate::metadata::FileVersion, TenetToolError> {
            let trimmed = v.trim().to_lowercase();
            if trimmed.eq_ignore_ascii_case("latest") {
                entry
                    .versions
                    .last()
                    .ok_or_else(|| TenetToolError("No versions available".to_string()))
            } else {
                let num_str = trimmed.strip_prefix('v').unwrap_or(&trimmed);
                let idx: usize = num_str
                    .parse()
                    .map_err(|_| TenetToolError(format!("Invalid version number: '{v}'")))?;
                if idx == 0 || idx > entry.versions.len() {
                    return Err(TenetToolError(format!(
                        "Version {idx} out of range (1–{})",
                        entry.versions.len()
                    )));
                }
                Ok(&entry.versions[idx - 1])
            }
        };

        let ver1 = resolve(&args.v1)?;
        let ver2 = resolve(&args.v2)?;

        if ver1.version_type == crate::metadata::VersionType::Deletion
            || ver2.version_type == crate::metadata::VersionType::Deletion
        {
            return Err(TenetToolError(
                "Cannot diff a deletion marker — choose a Snapshot version.".to_string(),
            ));
        }

        let content1 = crate::storage::read_blob(&self.tenet_dir, &ver1.hash)
            .map_err(|e| TenetToolError(e.to_string()))?;
        let content2 = crate::storage::read_blob(&self.tenet_dir, &ver2.hash)
            .map_err(|e| TenetToolError(e.to_string()))?;

        let text1 = String::from_utf8_lossy(&content1);
        let text2 = String::from_utf8_lossy(&content2);

        let lines1: Vec<&str> = text1.lines().collect();
        let lines2: Vec<&str> = text2.lines().collect();

        let mut out = format!(
            "Diff of '{}' between {} and {}:\n",
            args.file, args.v1, args.v2
        );

        let max = lines1.len().max(lines2.len());
        let mut changes = 0usize;

        for i in 0..max {
            match (lines1.get(i), lines2.get(i)) {
                (Some(a), Some(b)) if a != b => {
                    out.push_str(&format!("- {a}\n+ {b}\n"));
                    changes += 1;
                }
                (None, Some(b)) => {
                    out.push_str(&format!("+ {b}\n"));
                    changes += 1;
                }
                (Some(a), None) => {
                    out.push_str(&format!("- {a}\n"));
                    changes += 1;
                }
                _ => {}
            }
        }

        if changes == 0 {
            out.push_str("Files are identical — no differences found.");
        } else {
            out.push_str(&format!("\n{changes} line(s) changed."));
        }

        Ok(out)
    }
}
