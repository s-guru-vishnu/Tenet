# TENET — Time-Travel File System ⏳

A high-performance, systems-level file versioning tool built in Rust. TENET automatically tracks file changes in user-specified directories and allows restoring files to any previous state.

> *"What's happened, happened. Which is an expression of faith in the mechanics of the world."*
> — TENET

## Features

- **🔍 Real-time File Watching** — Monitor directories for changes using OS-level file system events
- **📸 Automatic Versioning** — Every file change is automatically captured and stored
- **⏪ Time Travel Restore** — Restore any file to any previous point in time
- **🧹 Smart Filtering** — `.tenetignore` support (like `.gitignore`) to skip unwanted files
- **💾 Content-Addressable Storage** — Deduplication via SHA-256 hashing
- **🔒 Crash-Safe** — Atomic writes prevent data corruption
- **⚡ High Performance** — Async I/O with Tokio, debounced events, efficient batching

## Installation & Usage

TENET operates in **Dual-Mode**: It features a rich **Desktop GUI** alongside a powerful **Command-Line Interface (CLI)**.

### 1. Desktop GUI
To run the Desktop application locally for development:
```bash
npm install
npm run tauri dev
```

To build a standalone executable for the Desktop application:
```bash
npm run tauri build
```

### 2. Global CLI Installation
To use TENET from your terminal across any directory in your system, install the core Rust binary globally:
```bash
cargo install --path src-tauri
```

Once installed globally, you can invoke the UI from anywhere by simply typing `tenet`, or use the CLI commands (e.g., `tenet watch .`)!

## Quick Start

### 1. Watch a Directory
```bash
tenet watch ./my-project
```

This will:
- Create a `.tenet/` directory inside `my-project/`
- Take an initial snapshot of all existing files
- Start monitoring for changes in real-time

### 2. View File History
```bash
tenet history src/main.rs
```

Shows all tracked versions with timestamps, hashes, and sizes.

### 3. Restore a File
```bash
# Restore to 1 hour ago
tenet restore src/main.rs@1h

# Restore to a specific time
tenet restore "src/main.rs@14:30"

# Restore to a specific date & time
tenet restore "src/main.rs@2024-01-15 14:30:00"

# Preview without modifying
tenet restore src/main.rs@1h --dry-run
```

### 4. Check Status
```bash
tenet status
```

Displays tracked files, version counts, and storage usage.

## Commands

| Command | Alias | Description |
|:--------|:------|:------------|
| `tenet watch <dir>` | `tenet w` | Start watching a directory |
| `tenet history <file>` | `tenet h` | Show version history |
| `tenet restore <file@time>` | `tenet r` | Restore to a point in time |
| `tenet status` | `tenet s` | Show tracking statistics |

## `.tenetignore`

Create a `.tenetignore` file in your watched directory to exclude files:

```gitignore
# Dependencies
node_modules/
target/

# Build output
dist/
build/

# Logs
*.log

# IDE files
.idea/
.vscode/
```

### Default Ignore Patterns
Even without a `.tenetignore`, TENET automatically ignores:
- `.git/`, `node_modules/`, `.cache/`, `target/`
- `.tenet/` (its own data)
- `*.log`, `*.tmp`, `*.swp`
- `.DS_Store`, `Thumbs.db`

## Architecture

```
src/
├── main.rs          # Entry point & command dispatch
├── cli/             # CLI argument parsing (clap)
├── watcher/         # File system monitoring (notify)
├── processor/       # Event filtering & .tenetignore
├── versioning/      # Snapshot/delta strategies
├── storage/         # Content-addressable blob store
├── metadata/        # Version history index
└── utils/           # Hashing, atomic writes, timestamps
```

### Storage Layout
```
.tenet/
├── metadata/
│   └── index.json       # Version history index
├── objects/
│   └── <sha256>.blob    # Content-addressable blobs
└── snapshots/
```

## Tech Stack

| Component | Technology |
|:----------|:-----------|
| Language | Rust |
| File Watcher | `notify` + `notify-debouncer-mini` |
| Async Runtime | `tokio` |
| CLI | `clap` (derive) |
| Serialization | `serde` + `serde_json` |
| Hashing | `sha2` (SHA-256) |
| Time | `chrono` |
| Error Handling | `anyhow` |

## License

MIT License — see [LICENSE](LICENSE)
