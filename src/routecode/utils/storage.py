import json
import os
import asyncio
import aiofiles
from pathlib import Path
from typing import Any, Dict, Optional


class AtomicJsonStore:
    """
    Unified, crash-safe JSON persistence layer.
    Ensures that writes are atomic by writing to a temporary file and
    renaming it to the target path.

    Supports context manager protocol for lifecycle scoping.
    """

    def __init__(self, path: Path):
        self.path = Path(path)
        self._ensure_dir()

    def _ensure_dir(self):
        self.path.parent.mkdir(parents=True, exist_ok=True)

    # ── Context manager protocol ─────────────────────────────────────────

    def __enter__(self):
        return self

    def __exit__(self, *args):
        self.cleanup_stale_temps()

    async def __aenter__(self):
        return self

    async def __aexit__(self, *args):
        self.cleanup_stale_temps()

    # ── Core API ─────────────────────────────────────────────────────────

    def load(self, default: Optional[Dict] = None) -> Dict[str, Any]:
        """Loads JSON data from the file. Returns default if file doesn't exist or is invalid."""
        if not self.path.exists():
            return default if default is not None else {}

        try:
            return json.loads(self.path.read_text(encoding="utf-8"))
        except (json.JSONDecodeError, Exception):
            return default if default is not None else {}

    async def load_async(self, default: Optional[Dict] = None) -> Dict[str, Any]:
        """Asynchronously loads JSON data from the file."""
        if not self.path.exists():
            return default if default is not None else {}

        try:
            async with aiofiles.open(self.path, mode="r", encoding="utf-8") as f:
                content = await f.read()
                return json.loads(content)
        except (json.JSONDecodeError, Exception):
            return default if default is not None else {}

    def _serialize(self, data: Dict[str, Any]) -> str:
        return json.dumps(data, indent=2, ensure_ascii=False)

    def save(self, data: Dict[str, Any]):
        """Saves data atomically to the file."""
        tmp_path = self.path.with_suffix(".tmp")
        try:
            tmp_path.write_text(self._serialize(data), encoding="utf-8")
            os.replace(tmp_path, self.path)
        except Exception as e:
            if tmp_path.exists():
                tmp_path.unlink()
            raise e

    async def save_async(self, data: Dict[str, Any]):
        """Asynchronously saves data atomically to the file."""
        tmp_path = self.path.with_suffix(".tmp")
        try:
            async with aiofiles.open(tmp_path, mode="w", encoding="utf-8") as f:
                await f.write(self._serialize(data))
            await asyncio.to_thread(os.replace, tmp_path, self.path)
        except Exception as e:
            if tmp_path.exists():
                await asyncio.to_thread(tmp_path.unlink)
            raise e

    def exists(self) -> bool:
        return self.path.exists()

    def delete(self):
        if self.path.exists():
            self.path.unlink()

    # ── Cleanup ──────────────────────────────────────────────────────────

    def cleanup_stale_temps(self, base_path: Optional[Path] = None):
        """
        Removes orphaned .tmp files left behind by crashed sessions.
        Call once at startup to clean up from previous incomplete writes.

        Args:
            base_path: Directory to scan. Defaults to the parent directory
                       of this store's path.
        """
        search_dir = base_path or self.path.parent
        if not search_dir.exists():
            return
        for tmp_file in search_dir.glob("*.tmp"):
            try:
                tmp_file.unlink()
            except Exception:
                pass
