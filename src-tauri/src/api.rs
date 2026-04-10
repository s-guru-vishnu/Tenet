use anyhow::{Context, Result};
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
        utils::find_tenet_dir(&file_path).context("Not in a TENET-tracked directory.")?;

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
        utils::find_tenet_dir(&file_path).context("Not in a TENET-tracked directory.")?;

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
            utils::find_tenet_dir(&current_dir).context("Not in a TENET-tracked directory.")?;

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
        utils::find_tenet_dir(&current_dir).context("Not in a TENET-tracked directory.")?;

    let watched_dir = tenet_dir
        .parent()
        .context("Invalid .tenet directory structure")?;
    let ignore_file = watched_dir.join(".tenetignore");

    std::fs::write(&ignore_file, rules).context("Failed to write ignore file")?;
    Ok(())
}
