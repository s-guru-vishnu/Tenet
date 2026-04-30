import subprocess
import json
from typing import Optional, Dict, Any, List

class TenetError(Exception):
    """Exception raised for errors in the Tenet SDK."""
    pass

class Tenet:
    """Python wrapper for the TENET File System binary."""

    def __init__(self, binary_path: str = "tenet"):
        self.binary_path = binary_path

    def _run_command(self, args: List[str]) -> str:
        """Helper to run a TENET CLI command and return its output."""
        try:
            result = subprocess.run(
                [self.binary_path] + args,
                capture_output=True,
                text=True,
                check=True
            )
            return result.stdout.strip()
        except subprocess.CalledProcessError as e:
            raise TenetError(f"TENET command failed: {e.stderr.strip()}")
        except FileNotFoundError:
            raise TenetError(
                f"TENET binary not found at '{self.binary_path}'. "
                "Ensure it is installed and in your PATH."
            )

    def start(self):
        """Start the TENET desktop application GUI."""
        try:
            subprocess.Popen([self.binary_path])
        except FileNotFoundError:
            raise TenetError("TENET binary not found. Cannot start GUI.")

    def watch(self, path: str) -> None:
        """
        Start watching a directory for changes.
        Note: This process blocks until interrupted.
        """
        self._run_command(["watch", path])

    def status(self) -> str:
        """Get the current tracking status."""
        return self._run_command(["status"])

    def history(self, file_path: str, limit: int = 20) -> str:
        """Get the version history for a specific file."""
        return self._run_command(["history", file_path, "--limit", str(limit)])

    def restore(self, file_path: str, timestamp: str, dry_run: bool = False) -> str:
        """
        Restore a file to a specific timestamp.
        Example timestamp: '1h', '2024-01-15 14:30:00'
        """
        target = f"{file_path}@{timestamp}"
        args = ["restore", target]
        if dry_run:
            args.append("--dry-run")
        return self._run_command(args)
