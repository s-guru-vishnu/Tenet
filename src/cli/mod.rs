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
