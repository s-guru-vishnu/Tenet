# tenet-cli

The official Node.js CLI wrapper for **TENET — Time-Travel File System**.

## Requirements

The TENET core binary must be installed on your system. 
You can install it via Cargo:
```bash
cargo install --path src-tauri
```

## Installation

```bash
npm install -g tenet-cli
```

## Usage

```bash
# Watch a directory
tenet-cli watch ./my-project

# View file history
tenet-cli history src/main.rs

# Restore a file
tenet-cli restore src/main.rs@1h

# Check status
tenet-cli status
```
