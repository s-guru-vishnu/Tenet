# tenet-sdk

Python SDK for **TENET — Time-Travel File System**.

## Requirements

The TENET core binary must be installed on your system.
Install it via Cargo:
```bash
cargo install --path src-tauri
```

## Installation

```bash
pip install -e .
```

## Example Usage

```python
from tenet import Tenet

t = Tenet()

# Open the Desktop App GUI
t.start()

# Check status
print(t.status())

# Get file history
print(t.history("src/main.rs"))

# Restore a file
t.restore("src/main.rs", "1h")

# Watch a directory (blocks)
t.watch("./project")
```
