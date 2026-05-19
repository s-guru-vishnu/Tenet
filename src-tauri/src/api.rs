use anyhow::Context;
use serde::Serialize;
use std::path::PathBuf;
use tauri::command;

use crate::metadata::MetadataIndex;
use crate::utils;
use crate::versioning;
use crate::watcher;
// use crate::processor::IgnoreRules;
use crate::storage;
use chrono::{DateTime, Utc};

// We create a serializable wrapper for errors
#[derive(Serialize)]
pub struct ApiError(String);

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError(err.to_string())
    }
}

type ApiResult<T> = std::result::Result<T, ApiError>;

#[derive(Serialize)]
pub struct StatusReport {
    pub watched_dir: String,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub file_count: usize,
    pub version_count: usize,
    pub blob_count: usize,
    pub storage_size: u64,
}

#[command]
pub async fn watch_directory(path: String) -> ApiResult<String> {
    let watched_dir = PathBuf::from(&path);

    if !watched_dir.is_dir() {
        return Err(anyhow::anyhow!("Path is not a valid directory").into());
    }

    // We spawn the watcher in the background so it doesn't block Tauri
    tokio::spawn(async move {
        if let Err(e) = watcher::watch_directory(&watched_dir).await {
            eprintln!("Watcher error: {}", e);
        }
    });

    Ok(format!("Started watching {}", path))
}

#[command]
pub async fn get_status(path: String) -> ApiResult<Option<StatusReport>> {
    let current_dir = PathBuf::from(&path);
    let tenet_dir = match utils::find_tenet_dir(&current_dir) {
        Some(dir) => dir,
        None => return Ok(None),
    };

    let watched_dir = tenet_dir
        .parent()
        .context("Invalid .tenet directory structure")?;
    let watched_dir_str = watched_dir.to_string_lossy().to_string();

    let metadata = MetadataIndex::load(&tenet_dir, &watched_dir_str)?;
    let blob_count = storage::blob_count(&tenet_dir);
    let storage_size = storage::total_storage_size(&tenet_dir);

    Ok(Some(StatusReport {
        watched_dir: watched_dir_str,
        created_at: metadata.created_at,
        last_updated: metadata.last_updated,
        file_count: metadata.file_count(),
        version_count: metadata.total_versions(),
        blob_count,
        storage_size,
    }))
}

#[command]
pub async fn get_history(file: String) -> ApiResult<crate::metadata::FileEntry> {
    let file_path = PathBuf::from(&file)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(&file));

    let tenet_dir =
        utils::find_tenet_dir(&file_path).context("Not in a TENEt - tracked directory.")?;

    let watched_dir = tenet_dir.parent().context("Invalid .tenet structure")?;
    let watched_dir_str = watched_dir.to_string_lossy().to_string();
    let metadata = MetadataIndex::load(&tenet_dir, &watched_dir_str)?;

    let rel_path =
        utils::relative_path(&file_path, watched_dir).unwrap_or_else(|_| file.replace('\\', "/"));

    let entry = metadata
        .get_history(&rel_path)
        .context("No history found for this file.")?;

    Ok(entry.clone())
}

#[command]
pub async fn restore_version(file: String, timestamp: String) -> ApiResult<String> {
    let time = utils::parse_timestamp(&timestamp)?;

    let file_path = PathBuf::from(&file)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(&file));

    let tenet_dir =
        utils::find_tenet_dir(&file_path).context("Not in a TENEt - tracked directory.")?;

    let watched_dir = tenet_dir.parent().context("Invalid .tenet structure")?;
    let watched_dir_str = watched_dir.to_string_lossy().to_string();
    let metadata = MetadataIndex::load(&tenet_dir, &watched_dir_str)?;

    let rel_path =
        utils::relative_path(&file_path, watched_dir).unwrap_or_else(|_| file.replace('\\', "/"));

    let result = versioning::restore_file(&rel_path, &time, watched_dir, &tenet_dir, &metadata)?;

    Ok(result)
}

#[command]
pub async fn get_tracked_files(path: String) -> ApiResult<Vec<String>> {
    let current_dir = PathBuf::from(&path);
    let tenet_dir = match utils::find_tenet_dir(&current_dir) {
        Some(dir) => dir,
        None => return Ok(vec![]),
    };

    let watched_dir = tenet_dir
        .parent()
        .context("Invalid .tenet directory structure")?;
    let watched_dir_str = watched_dir.to_string_lossy().to_string();

    let metadata = MetadataIndex::load(&tenet_dir, &watched_dir_str)?;
    let files = metadata
        .tracked_files()
        .into_iter()
        .map(String::from)
        .collect();

    Ok(files)
}

#[command]
pub async fn get_file_content(path: String, hash: Option<String>) -> ApiResult<String> {
    if let Some(h) = hash {
        // Read from blob
        let mut current_dir = PathBuf::from(&path);
        // If the path is a file, use its parent to find .tenet
        if current_dir.is_file() {
            if let Some(parent) = current_dir.parent() {
                current_dir = parent.to_path_buf();
            }
        }

        let tenet_dir =
            utils::find_tenet_dir(&current_dir).context("Not in a TENEt - tracked directory.")?;

        let content_bytes = storage::read_blob(&tenet_dir, &h)?;
        let content = String::from_utf8(content_bytes)
            .map_err(|_| anyhow::anyhow!("File is not valid UTF-8"))?;
        Ok(content)
    } else {
        // Read current file
        let file_path = PathBuf::from(&path);
        if !file_path.exists() {
            return Ok(String::new());
        }
        let content_bytes = std::fs::read(&file_path).context("Failed to read file")?;
        let content = String::from_utf8(content_bytes)
            .map_err(|_| anyhow::anyhow!("File is not valid UTF-8"))?;
        Ok(content)
    }
}

#[command]
pub async fn get_ignore_rules(path: String) -> ApiResult<String> {
    let current_dir = PathBuf::from(&path);
    let tenet_dir = match utils::find_tenet_dir(&current_dir) {
        Some(dir) => dir,
        None => return Ok(String::new()),
    };

    let watched_dir = tenet_dir
        .parent()
        .context("Invalid .tenet directory structure")?;
    let ignore_file = watched_dir.join(".tenetignore");

    if ignore_file.exists() {
        let content = std::fs::read_to_string(ignore_file).context("Failed to read ignore file")?;
        Ok(content)
    } else {
        Ok(String::new())
    }
}

#[command]
pub async fn save_ignore_rules(path: String, rules: String) -> ApiResult<()> {
    let current_dir = PathBuf::from(&path);
    let tenet_dir =
        utils::find_tenet_dir(&current_dir).context("Not in a TENEt - tracked directory.")?;

    let watched_dir = tenet_dir
        .parent()
        .context("Invalid .tenet directory structure")?;
    let ignore_file = watched_dir.join(".tenetignore");

    std::fs::write(&ignore_file, rules).context("Failed to write ignore file")?;
    Ok(())
}

// ─── AI Agent ────────────────────────────────────────────────────────────────

/// Runs the TENET AI agent with a natural-language query.
///
/// The agent is backed by rig.rs (Groq / Llama) and has access to all
/// four TENET tools: get_history, restore_version, list_files, diff_versions.
///
/// # Arguments
/// * `query`   — Natural language command (e.g. "Restore main.rs from yesterday")
/// * `path`    — Current watched directory path (so we can locate `.tenet/`)
/// * `api_key` — Groq API key provided by the user in Settings
#[command]
pub async fn run_agent(query: String, path: String, api_key: String, provider: String) -> ApiResult<String> {
    if api_key.trim().is_empty() {
        return Err(anyhow::anyhow!(
            "API key is not set. Please provide it in the chat interface."
        )
        .into());
    }

    let current_dir = PathBuf::from(&path);
    let tenet_dir = utils::find_tenet_dir(&current_dir)
        .context("Not in a TENET-tracked directory. Watch a directory first.")?;

    let watched_dir = tenet_dir
        .parent()
        .context("Invalid .tenet directory structure")?
        .to_path_buf();

    let result = crate::agent::run_agent(&query, watched_dir, tenet_dir, &api_key, &provider)
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        
    Ok(result)
}

/// Compares two stored versions of a file and returns a unified diff string.
/// Used by the frontend Diff Viewer and the AI agent's diff_versions tool.
#[command]
pub async fn diff_versions(
    file: String,
    path: String,
    v1: String,
    v2: String,
) -> ApiResult<String> {
    let current_dir = PathBuf::from(&path);
    let tenet_dir = utils::find_tenet_dir(&current_dir)
        .context("Not in a TENET-tracked directory.")?;

    let watched_dir = tenet_dir
        .parent()
        .context("Invalid .tenet directory structure")?;
    let watched_dir_str = watched_dir.to_string_lossy().to_string();

    let metadata = MetadataIndex::load(&tenet_dir, &watched_dir_str)?;
    let entry = metadata
        .get_history(&file)
        .context(format!("No history found for '{file}'"))?;

    let resolve = |v: &str| -> anyhow::Result<&crate::metadata::FileVersion> {
        if v.eq_ignore_ascii_case("latest") {
            entry.versions.last().context("No versions available")
        } else {
            let idx: usize = v.parse().context("Invalid version number")?;
            if idx == 0 || idx > entry.versions.len() {
                anyhow::bail!("Version {idx} out of range");
            }
            Ok(&entry.versions[idx - 1])
        }
    };

    let ver1 = resolve(&v1)?;
    let ver2 = resolve(&v2)?;

    let c1 = crate::storage::read_blob(&tenet_dir, &ver1.hash)?;
    let c2 = crate::storage::read_blob(&tenet_dir, &ver2.hash)?;

    let text1 = String::from_utf8_lossy(&c1);
    let text2 = String::from_utf8_lossy(&c2);

    let lines1: Vec<&str> = text1.lines().collect();
    let lines2: Vec<&str> = text2.lines().collect();

    let mut out = String::new();
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
        out = "No differences found.".to_string();
    }

    Ok(out)
}
