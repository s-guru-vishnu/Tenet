//! # CLI Module
//!
//! Defines the command-line interface for TENET using the `clap` derive API.
//!
//! ## Commands
//! - `tenet watch <directory>` — Start watching a directory for changes
//! - `tenet history <file>` — Show version history for a file
//! - `tenet restore <file@time>` — Restore a file to a previous version
//! - `tenet status` — Show current tracking status and statistics

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// TENET — Time-Travel File System
///
/// Track file changes in real-time and restore any file to any
/// previous version. Like having an undo button for your entire
/// filesystem.
#[derive(Parser, Debug)]
#[command(
    name = "tenet",
    version,
    about = "⏳ TENET — Time-Travel File System",
    long_about = "Track file changes in real-time and restore any file to any \
                   previous version.\nLike having an undo button for your entire filesystem.",
    author = "S Guru Vishnu"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Available subcommands for the TENET CLI.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Watch a directory for file changes and track versions automatically
    ///
    /// Starts monitoring the specified directory recursively. All file
    /// changes (creates, modifications, deletions) are tracked and
    /// versioned automatically. Press Ctrl+C to stop watching.
    #[command(alias = "w")]
    Watch {
        /// The directory to watch for changes
        #[arg(value_name = "DIRECTORY")]
        directory: PathBuf,
    },

    /// Show version history for a specific file
    ///
    /// Displays all tracked versions of the specified file, including
    /// timestamps, content hashes, and file sizes.
    #[command(alias = "h")]
    History {
        /// The file to show history for
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// Maximum number of versions to display
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },

    /// Restore a file to a previous version
    ///
    /// Restores a file to the version closest to the specified time.
    ///
    /// Time formats supported:
    ///   - Relative: 1h, 30m, 2d (ago from now)
    ///   - Time only: 14:30 (today)
    ///   - Full: 2024-01-15 14:30:00
    ///
    /// Examples:
    ///   tenet restore main.rs@1h
    ///   tenet restore "src/lib.rs@2024-01-15 14:30:00"
    #[command(alias = "r")]
    Restore {
        /// Target in the format file@time
        ///
        /// The file path and timestamp separated by '@'.
        /// Example: main.rs@1h, "src/lib.rs@14:30"
        #[arg(value_name = "FILE@TIME")]
        target: String,

        /// Preview the restore without actually modifying the file
        #[arg(long, short = 'n')]
        dry_run: bool,
    },

    /// Show current tracking status and statistics
    ///
    /// Displays information about the watched directory, including
    /// the number of tracked files, total versions, and storage usage.
    #[command(alias = "s")]
    Status,
}

/// Parses a restore target string into (file_path, time_string).
///
/// The format is `file@time`, where `@` is the separator.
/// The last `@` in the string is used as the separator to support
/// file paths that might (rarely) contain `@`.
///
/// # Examples
/// ```
/// let (file, time) = parse_restore_target("main.rs@1h").unwrap();
/// assert_eq!(file, "main.rs");
/// assert_eq!(time, "1h");
/// ```
pub fn parse_restore_target(target: &str) -> anyhow::Result<(String, String)> {
    // Find the last '@' to split on
    let at_pos = target
        .rfind('@')
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Invalid restore target: '{}'\n\
                 Expected format: file@time\n\
                 Examples:\n  \
                   tenet restore main.rs@1h\n  \
                   tenet restore \"src/lib.rs@14:30\"\n  \
                   tenet restore \"file.txt@2024-01-15 14:30:00\"",
                target
            )
        })?;

    let file = target[..at_pos].to_string();
    let time = target[at_pos + 1..].to_string();

    if file.is_empty() {
        anyhow::bail!("File path cannot be empty in restore target: '{}'", target);
    }
    if time.is_empty() {
        anyhow::bail!("Time cannot be empty in restore target: '{}'", target);
    }

    Ok((file, time))
}

/// Executes the CLI commands
pub async fn run_cli() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Watch { directory } => {
            crate::watcher::watch_directory(&directory).await?;
        }
        Commands::History { file, limit } => {
            let file_path = file.canonicalize().unwrap_or(file.to_path_buf());
            let tenet_dir = crate::utils::find_tenet_dir(&file_path)
                .ok_or_else(|| anyhow::anyhow!("Not in a TENET-tracked directory."))?;
            let watched_dir = tenet_dir.parent().unwrap();
            let watched_dir_str = watched_dir.to_string_lossy().to_string();
            let metadata = crate::metadata::MetadataIndex::load(&tenet_dir, &watched_dir_str)?;
            let rel_path = crate::utils::relative_path(&file_path, watched_dir)
                .unwrap_or_else(|_| file_path.to_string_lossy().replace('\\', "/"));
            
            let entry = metadata
                .get_history(&rel_path)
                .ok_or_else(|| anyhow::anyhow!("No history found for this file."))?;
            
            println!("History for {}:", rel_path);
            let versions: Vec<_> = entry.versions.iter().rev().take(limit).collect();
            for v in versions {
                println!(
                    "- [{}]: {} (size: {} bytes)",
                    v.timestamp.format("%Y-%m-%d %H:%M:%S"),
                    v.hash.chars().take(8).collect::<String>(),
                    v.size
                );
            }
        }
        Commands::Restore { target, dry_run } => {
            let (file, time_str) = parse_restore_target(&target)?;
            let parsed_time = crate::utils::parse_timestamp(&time_str)?;
            
            let file_path = std::path::PathBuf::from(&file).canonicalize().unwrap_or_else(|_| std::path::PathBuf::from(&file));
            let tenet_dir = crate::utils::find_tenet_dir(&file_path)
                .ok_or_else(|| anyhow::anyhow!("Not in a TENET-tracked directory."))?;
            let watched_dir = tenet_dir.parent().unwrap();
            let watched_dir_str = watched_dir.to_string_lossy().to_string();
            let metadata = crate::metadata::MetadataIndex::load(&tenet_dir, &watched_dir_str)?;
            let rel_path = crate::utils::relative_path(&file_path, watched_dir)
                .unwrap_or_else(|_| file.replace('\\', "/"));
            
            if dry_run {
                println!(
                    "Would restore {} to time nearest to {}",
                    rel_path,
                    parsed_time.format("%Y-%m-%d %H:%M:%S")
                );
            } else {
                let res = crate::versioning::restore_file(&rel_path, &parsed_time, watched_dir, &tenet_dir, &metadata)?;
                println!("Success! {}", res);
            }
        }
        Commands::Status => {
            let current_dir = std::env::current_dir()?;
            let tenet_dir = crate::utils::find_tenet_dir(&current_dir)
                .ok_or_else(|| anyhow::anyhow!("Not in a TENET-tracked directory."))?;
            let watched_dir = tenet_dir.parent().unwrap();
            let watched_dir_str = watched_dir.to_string_lossy().to_string();
            let metadata = crate::metadata::MetadataIndex::load(&tenet_dir, &watched_dir_str)?;
            
            println!("Status for: {}", watched_dir_str);
            println!("Tracked files: {}", metadata.file_count());
            println!("Total versions: {}", metadata.total_versions());
            println!("Storage size: {} bytes", crate::storage::total_storage_size(&tenet_dir));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_restore_target() {
        let (file, time) = parse_restore_target("main.rs@1h").unwrap();
        assert_eq!(file, "main.rs");
        assert_eq!(time, "1h");

        let (file, time) = parse_restore_target("src/lib.rs@14:30").unwrap();
        assert_eq!(file, "src/lib.rs");
        assert_eq!(time, "14:30");

        assert!(parse_restore_target("no_at_sign").is_err());
        assert!(parse_restore_target("@no_file").is_err());
        assert!(parse_restore_target("no_time@").is_err());
    }
}
