import json
import os
from pathlib import Path
from typing import Any, Dict, Optional

class AtomicJsonStore:
    """
    Unified, crash-safe JSON persistence layer.
    Ensures that writes are atomic by writing to a temporary file and 
    renaming it to the target path.
    """
    def __init__(self, path: Path):
        self.path = Path(path)
        self._ensure_dir()

    def _ensure_dir(self):
        self.path.parent.mkdir(parents=True, exist_ok=True)

    def load(self, default: Optional[Dict] = None) -> Dict[str, Any]:
        """Loads JSON data from the file. Returns default if file doesn't exist or is invalid."""
        if not self.path.exists():
            return default if default is not None else {}
        
        try:
            return json.loads(self.path.read_text(encoding="utf-8"))
        except (json.JSONDecodeError, Exception):
            # In an industry solution, we might want to log this error
            return default if default is not None else {}

    def save(self, data: Dict[str, Any]):
        """Saves data atomically to the file."""
        tmp_path = self.path.with_suffix(".tmp")
        try:
            # Write to temp file
            tmp_path.write_text(json.dumps(data, indent=2, ensure_ascii=False), encoding="utf-8")
            # Atomic rename (replace target if it exists)
            # On Windows, os.replace is atomic if target and source are on the same drive.
            os.replace(tmp_path, self.path)
        except Exception as e:
            # Cleanup temp file if write failed
            if tmp_path.exists():
                tmp_path.unlink()
            raise e

    def exists(self) -> bool:
        return self.path.exists()

    def delete(self):
        if self.path.exists():
            self.path.unlink()
