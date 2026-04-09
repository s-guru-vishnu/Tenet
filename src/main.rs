//! # TENET — Time-Travel File System
//!
//! A high-performance systems-level application that tracks file changes
//! in user-specified directories and allows restoring files to any
//! previous state.
//!
//! ## Architecture
//! ```text
//! ┌─────────┐     ┌───────────┐     ┌────────────┐     ┌─────────┐
//! │ Watcher │ ──> │ Processor │ ──> │ Versioning │ ──> │ Storage │
//! │ (notify)│     │ (filter)  │     │ (snapshot)  │     │ (blobs) │
//! └─────────┘     └───────────┘     └────────────┘     └─────────┘
//!                                          │
//!                                   ┌──────┴──────┐
//!                                   │  Metadata   │
//!                                   │  (index)    │
//!                                   └─────────────┘
//! ```
//!
//! ## Modules
//! - **`watcher`** — OS-level file system monitoring with debouncing
//! - **`processor`** — Event filtering, `.tenetignore` support
//! - **`versioning`** — Snapshot/delta storage strategies
//! - **`storage`** — Content-addressable blob store
//! - **`metadata`** — Version history index management
//! - **`cli`** — Command-line interface (clap)
//! - **`utils`** — Hashing, atomic writes, timestamp utilities

mod cli;
mod metadata;
mod processor;
mod storage;
mod utils;
mod versioning;
mod watcher;

use anyhow::{Context, Result};
use chrono::Utc;
use clap::Parser;
use colored::Colorize;
use std::path::Path;

use cli::{Cli, Commands};
use metadata::MetadataIndex;
use processor::IgnoreRules;

/// Application entry point.
///
/// Initializes the tokio async runtime and dispatches to the
/// appropriate command handler based on CLI arguments.
#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Watch { directory } => {
            cmd_watch(&directory).await?;
        }
        Commands::History { file, limit } => {
            cmd_history(&file, limit)?;
        }
        Commands::Restore { target, dry_run } => {
            cmd_restore(&target, dry_run)?;
        }
        Commands::Status => {
            cmd_status()?;
        }
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// Command Handlers
// ═══════════════════════════════════════════════════════════════════

/// Handler for `tenet watch <directory>`
///
/// Starts the file system watcher on the specified directory.
/// Creates the `.tenet/` structure if it doesn't exist, takes an
/// initial snapshot, then monitors for changes in real-time.
async fn cmd_watch(directory: &Path) -> Result<()> {
    watcher::watch_directory(directory).await
}

/// Handler for `tenet history <file>`
///
/// Displays the version history of a specific file in a formatted table.
/// Shows timestamps, content hashes, file sizes, and version types.
fn cmd_history(file: &Path, limit: usize) -> Result<()> {
    // Resolve the file path
    let file_path = file
        .canonicalize()
        .unwrap_or_else(|_| std::env::current_dir().unwrap().join(file));

    // Find the .tenet directory by searching upward
    let tenet_dir = utils::find_tenet_dir(&file_path)
        .context("Not in a TENET-tracked directory. Run 'tenet watch <dir>' first.")?;

    let watched_dir = tenet_dir
        .parent()
        .context("Invalid .tenet directory structure")?;

    // Load metadata
    let watched_dir_str = watched_dir.to_string_lossy().to_string();
    let metadata = MetadataIndex::load(&tenet_dir, &watched_dir_str)?;

    // Get relative path for lookup
    let rel_path = utils::relative_path(&file_path, watched_dir)
        .or_else(|_| {
            // If exact path fails, try using the file argument directly
            Ok::<String, anyhow::Error>(file.to_string_lossy().replace('\\', "/"))
        })?;

    // Look up history
    let entry = metadata.get_history(&rel_path).with_context(|| {
        format!(
            "No history found for '{}'\n\
             Tip: Make sure you're using the path relative to the watched directory.",
            rel_path
        )
    })?;

    // Print header
    println!();
    println!(
        "{}  {}",
        "📜 Version History:".bright_cyan(),
        rel_path.bright_yellow()
    );
    println!(
        "{}",
        "─".repeat(70).bright_black()
    );

    // Print column headers
    println!(
        "  {:<4} {:<22} {:<14} {:<10} {}",
        "#".bright_white(),
        "Timestamp".bright_white(),
        "Hash".bright_white(),
        "Size".bright_white(),
        "Type".bright_white(),
    );
    println!(
        "{}",
        "─".repeat(70).bright_black()
    );

    // Print versions (most recent first, limited)
    let versions: Vec<_> = entry.versions.iter().rev().take(limit).collect();
    for (idx, version) in versions.iter().enumerate() {
        let version_num = entry.versions.len() - idx;
        let timestamp = utils::format_timestamp(&version.timestamp);
        let hash_short = if version.hash.is_empty() {
            "—".to_string()
        } else {
            format!("{}…", &version.hash[..10])
        };
        let size_str = format_size(version.size);
        let type_str = match version.version_type {
            metadata::VersionType::Snapshot => "snapshot".green(),
            metadata::VersionType::Delta => "delta".blue(),
            metadata::VersionType::Deletion => "deleted".red(),
        };

        println!(
            "  {:<4} {:<22} {:<14} {:<10} {}",
            format!("v{}", version_num).bright_blue(),
            timestamp,
            hash_short.bright_black(),
            size_str,
            type_str,
        );
    }

    println!(
        "{}",
        "─".repeat(70).bright_black()
    );
    println!(
        "  {} version(s) total",
        entry.versions.len().to_string().bright_cyan()
    );
    println!();

    Ok(())
}

/// Handler for `tenet restore <file@time>`
///
/// Parses the target, finds the appropriate version, and restores
/// the file to that state. Supports dry-run mode for previewing.
fn cmd_restore(target: &str, dry_run: bool) -> Result<()> {
    // Parse the file@time format
    let (file_str, time_str) = cli::parse_restore_target(target)?;

    // Parse the timestamp
    let timestamp = utils::parse_timestamp(&time_str)?;

    // Resolve the file path to find tenet dir
    let file_path = Path::new(&file_str)
        .canonicalize()
        .unwrap_or_else(|_| std::env::current_dir().unwrap().join(&file_str));

    // Determine paths — try to find .tenet from the target file's path
    let tenet_dir = utils::find_tenet_dir(&file_path)
        .context("Not in a TENET-tracked directory. Run 'tenet watch <dir>' first.")?;

    let watched_dir = tenet_dir
        .parent()
        .context("Invalid .tenet directory structure")?;

    // Load metadata
    let watched_dir_str = watched_dir.to_string_lossy().to_string();
    let metadata = MetadataIndex::load(&tenet_dir, &watched_dir_str)?;

    // Get relative path for lookup
    let rel_path = utils::relative_path(&file_path, watched_dir)
        .or_else(|_| {
            Ok::<String, anyhow::Error>(file_str.replace('\\', "/"))
        })?;
    if dry_run {
        // Preview mode
        println!();
        println!("{}", "🔍 Dry Run — Preview Only".bright_yellow());
        println!();

        match metadata.find_version_at(&rel_path, &timestamp) {
            Some(version) => {
                println!("  Would restore: {}", rel_path.bright_cyan());
                println!(
                    "  To version:    {} ({})",
                    utils::format_timestamp(&version.timestamp),
                    match version.version_type {
                        metadata::VersionType::Snapshot => "snapshot",
                        metadata::VersionType::Delta => "delta",
                        metadata::VersionType::Deletion => "deletion (cannot restore)",
                    }
                );
                println!(
                    "  Hash:          {}",
                    if version.hash.is_empty() {
                        "—".to_string()
                    } else {
                        version.hash[..16].to_string()
                    }
                    .bright_black()
                );
                println!("  Size:          {}", format_size(version.size));
                println!();
                println!(
                    "  {}",
                    "Run without --dry-run to apply.".bright_black()
                );
            }
            None => {
                println!(
                    "  {} No version found for '{}' at or before {}",
                    "✗".red(),
                    rel_path,
                    utils::format_timestamp(&timestamp)
                );
            }
        }
    } else {
        // Actually restore
        println!();
        let result = versioning::restore_file(
            &rel_path,
            &timestamp,
            watched_dir,
            &tenet_dir,
            &metadata,
        )?;
        println!("  {} {}", "✅".green(), result.green());
        println!();
    }

    Ok(())
}

/// Handler for `tenet status`
///
/// Shows a summary of the TENET tracking state for the current directory.
fn cmd_status() -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let tenet_dir = utils::find_tenet_dir(&current_dir)
        .context("Not in a TENET-tracked directory. Run 'tenet watch <dir>' first.")?;

    let watched_dir = tenet_dir
        .parent()
        .context("Invalid .tenet directory structure")?;

    let watched_dir_str = watched_dir.to_string_lossy().to_string();
    let metadata = MetadataIndex::load(&tenet_dir, &watched_dir_str)?;
    let _ignore_rules = IgnoreRules::load(watched_dir);

    // Gather statistics
    let file_count = metadata.file_count();
    let version_count = metadata.total_versions();
    let blob_count = storage::blob_count(&tenet_dir);
    let storage_size = storage::total_storage_size(&tenet_dir);

    // Print status
    println!();
    println!(
        "{}",
        "╔══════════════════════════════════════════════╗".bright_cyan()
    );
    println!(
        "{}",
        "║          ⏳ TENET — Status Report ⏳          ║".bright_cyan()
    );
    println!(
        "{}",
        "╚══════════════════════════════════════════════╝".bright_cyan()
    );
    println!();

    println!(
        "  {} {}",
        "Watched Directory:".bright_white(),
        watched_dir.display().to_string().bright_yellow()
    );
    println!(
        "  {} {}",
        "Index Created:    ".bright_white(),
        utils::format_timestamp(&metadata.created_at).bright_yellow()
    );
    println!(
        "  {} {}",
        "Last Updated:     ".bright_white(),
        utils::format_timestamp(&metadata.last_updated).bright_yellow()
    );
    println!();

    println!("  {}", "📊 Statistics".bright_cyan());
    println!(
        "{}",
        "  ─────────────────────────────────────────────".bright_black()
    );
    println!(
        "  {:<25} {}",
        "Tracked Files:".bright_white(),
        file_count.to_string().bright_green()
    );
    println!(
        "  {:<25} {}",
        "Total Versions:".bright_white(),
        version_count.to_string().bright_green()
    );
    println!(
        "  {:<25} {}",
        "Stored Blobs:".bright_white(),
        blob_count.to_string().bright_green()
    );
    println!(
        "  {:<25} {}",
        "Storage Used:".bright_white(),
        format_size(storage_size).bright_green()
    );

    // Show recent files (up to 10)
    let tracked = metadata.tracked_files();
    if !tracked.is_empty() {
        println!();
        println!("  {}", "📁 Tracked Files".bright_cyan());
        println!(
            "{}",
            "  ─────────────────────────────────────────────".bright_black()
        );

        let display_count = tracked.len().min(15);
        for path in tracked.iter().take(display_count) {
            let latest = metadata.get_latest_version(path);
            let version_count = metadata
                .get_history(path)
                .map(|e| e.versions.len())
                .unwrap_or(0);

            let status_icon = match latest.map(|v| &v.version_type) {
                Some(metadata::VersionType::Deletion) => "✗".red(),
                _ => "●".green(),
            };

            println!(
                "  {} {:<40} ({} version{})",
                status_icon,
                path,
                version_count,
                if version_count == 1 { "" } else { "s" }
            );
        }

        if tracked.len() > display_count {
            println!(
                "  {} ... and {} more",
                "".bright_black(),
                (tracked.len() - display_count).to_string().bright_black()
            );
        }
    }

    println!();

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// Helper Functions
// ═══════════════════════════════════════════════════════════════════

/// Formats a byte count into a human-readable size string.
///
/// Examples: "0 B", "1.5 KB", "3.2 MB", "1.0 GB"
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
