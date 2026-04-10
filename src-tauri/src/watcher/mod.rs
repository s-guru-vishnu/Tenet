//! # File Watcher Module
//!
//! Monitors directories for file system changes using the `notify` crate
//! with debouncing via `notify-debouncer-mini`.
//!
//! ## Design
//! - Uses OS-level file watching (`ReadDirectoryChangesW` on Windows,
//!   `inotify` on Linux, `FSEvents` on macOS) for efficiency
//! - Debounces rapid events (500ms window) to avoid processing
//!   intermediate states during saves
//! - Runs in an async context with `tokio` for non-blocking operation
//! - Supports graceful shutdown via Ctrl+C signal handling
//!
//! ## Flow
//! ```text
//! OS File Events → notify → debouncer (500ms) → channel → processor
//! ```

use crate::metadata::MetadataIndex;
use crate::processor::{self, IgnoreRules};
use crate::utils;
use crate::versioning;
use anyhow::{Context, Result};
use colored::Colorize;
use notify_debouncer_mini::{new_debouncer, notify, DebouncedEventKind};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

/// Duration for the debounce window.
///
/// Events occurring within this window are batched together.
/// 500ms strikes a good balance between responsiveness and
/// avoiding processing intermediate save states.
const DEBOUNCE_DURATION: Duration = Duration::from_millis(500);

/// Starts watching a directory for file changes.
///
/// This is the main loop for the `tenet watch` command. It:
/// 1. Initializes the `.tenet/` directory structure
/// 2. Creates an initial snapshot of all existing files
/// 3. Starts the file system watcher with debouncing
/// 4. Processes events in a loop until Ctrl+C
///
/// # Arguments
/// * `dir` - The directory to watch
///
/// # Errors
/// Returns an error if the directory doesn't exist, the watcher
/// can't be created, or critical I/O operations fail.
pub async fn watch_directory(dir: &Path) -> Result<()> {
    // Validate the directory exists
    let watched_dir = dir
        .canonicalize()
        .with_context(|| format!("Directory not found: {}", dir.display()))?;

    if !watched_dir.is_dir() {
        anyhow::bail!("'{}' is not a directory", watched_dir.display());
    }

    // Initialize .tenet/ directory structure
    let tenet_dir = utils::ensure_tenet_dir(&watched_dir)?;

    // Load or create metadata index
    let watched_dir_str = watched_dir.to_string_lossy().to_string();
    let mut metadata = MetadataIndex::load(&tenet_dir, &watched_dir_str)?;

    // Load ignore rules
    let ignore_rules = IgnoreRules::load(&watched_dir);

    // Print startup banner
    print_banner(&watched_dir);

    // Create initial snapshot of existing files
    println!(
        "{}",
        "📸 Creating initial snapshot of existing files...".cyan()
    );
    let initial_count = versioning::create_initial_snapshot(
        &watched_dir,
        &tenet_dir,
        &mut metadata,
        &ignore_rules,
    )?;
    println!(
        "{}",
        format!("✅ Snapshotted {} files", initial_count).green()
    );

    // Set up the debounced file watcher
    let (tx, rx) = mpsc::channel();

    let mut debouncer =
        new_debouncer(DEBOUNCE_DURATION, tx).context("Failed to create file watcher")?;

    // Start watching recursively
    debouncer
        .watcher()
        .watch(&watched_dir, notify::RecursiveMode::Recursive)
        .with_context(|| format!("Failed to watch directory: {}", watched_dir.display()))?;

    println!(
        "{}",
        format!("👁️  Watching: {}", watched_dir.display()).bright_blue()
    );
    println!("{}", "Press Ctrl+C to stop watching.\n".bright_black());

    // Main event processing loop
    // We use a separate thread for the blocking channel receiver
    // and communicate back to the async context
    let watched_dir_clone = watched_dir.clone();
    let tenet_dir_clone = tenet_dir.clone();

    // Process events in the current async task using blocking spawn
    let handle = tokio::task::spawn_blocking(move || {
        process_event_loop(
            rx,
            &watched_dir_clone,
            &tenet_dir_clone,
            &mut metadata,
            &ignore_rules,
        )
    });

    // Wait for Ctrl+C or the event loop to end
    tokio::select! {
        result = handle => {
            result.context("Event processing task panicked")??;
        }
        _ = tokio::signal::ctrl_c() => {
            println!("\n{}", "⏹️  Stopping watcher... Goodbye!".yellow());
        }
    }

    Ok(())
}

/// The blocking event processing loop.
///
/// Receives debounced events from the channel and dispatches them
/// to the processor. Runs until the channel is closed or an
/// unrecoverable error occurs.
fn process_event_loop(
    rx: mpsc::Receiver<Result<Vec<notify_debouncer_mini::DebouncedEvent>, notify::Error>>,
    watched_dir: &Path,
    tenet_dir: &Path,
    metadata: &mut MetadataIndex,
    ignore_rules: &IgnoreRules,
) -> Result<()> {
    loop {
        match rx.recv() {
            Ok(Ok(events)) => {
                // Extract unique file paths from the batch
                let paths: Vec<PathBuf> = events
                    .into_iter()
                    .filter(|e| e.kind == DebouncedEventKind::Any)
                    .map(|e| e.path)
                    .collect::<std::collections::HashSet<_>>()
                    .into_iter()
                    .collect();

                if paths.is_empty() {
                    continue;
                }

                // Process the batch
                match processor::process_events(
                    &paths,
                    watched_dir,
                    tenet_dir,
                    metadata,
                    ignore_rules,
                ) {
                    Ok(count) => {
                        if count > 0 {
                            println!(
                                "{}",
                                format!(
                                    "📝 Processed {} file change(s) at {}",
                                    count,
                                    chrono::Local::now().format("%H:%M:%S")
                                )
                                .green()
                            );

                            // Print which files changed
                            for path in &paths {
                                if !ignore_rules.should_ignore(path, watched_dir) {
                                    if let Ok(rel) = utils::relative_path(path, watched_dir) {
                                        if path.exists() {
                                            println!("   {} {}", "→".bright_blue(), rel);
                                        } else {
                                            println!(
                                                "   {} {} {}",
                                                "✗".red(),
                                                rel,
                                                "(deleted)".bright_black()
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("{}", format!("⚠️  Error processing events: {}", e).red());
                    }
                }
            }
            Ok(Err(e)) => {
                eprintln!("{}", format!("⚠️  Watcher error: {:?}", e).red());
            }
            Err(_) => {
                // Channel closed — watcher was dropped
                break;
            }
        }
    }

    Ok(())
}

/// Prints the TENET startup banner.
fn print_banner(watched_dir: &Path) {
    println!();
    println!(
        "{}",
        "╔══════════════════════════════════════════════╗".bright_cyan()
    );
    println!(
        "{}",
        "║         ⏳ TENET — Time-Travel FS ⏳         ║".bright_cyan()
    );
    println!(
        "{}",
        "╚══════════════════════════════════════════════╝".bright_cyan()
    );
    println!();
    println!(
        "  {} {}",
        "Directory:".bright_white(),
        watched_dir.display().to_string().bright_yellow()
    );
    println!(
        "  {} {}",
        "Started:  ".bright_white(),
        chrono::Local::now()
            .format("%Y-%m-%d %H:%M:%S")
            .to_string()
            .bright_yellow()
    );
    println!();
}
