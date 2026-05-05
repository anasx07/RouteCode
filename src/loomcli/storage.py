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
            return default if default is not None else {}

    async def load_async(self, default: Optional[Dict] = None) -> Dict[str, Any]:
        """Asynchronously loads JSON data from the file."""
        if not self.path.exists():
            return default if default is not None else {}
        
        try:
            async with aiofiles.open(self.path, mode='r', encoding='utf-8') as f:
                content = await f.read()
                return json.loads(content)
        except (json.JSONDecodeError, Exception):
            return default if default is not None else {}

    def save(self, data: Dict[str, Any]):
        """Saves data atomically to the file."""
        tmp_path = self.path.with_suffix(".tmp")
        try:
            tmp_path.write_text(json.dumps(data, indent=2, ensure_ascii=False), encoding="utf-8")
            os.replace(tmp_path, self.path)
        except Exception as e:
            if tmp_path.exists():
                tmp_path.unlink()
            raise e

    async def save_async(self, data: Dict[str, Any]):
        """Asynchronously saves data atomically to the file."""
        tmp_path = self.path.with_suffix(".tmp")
        try:
            async with aiofiles.open(tmp_path, mode='w', encoding='utf-8') as f:
                await f.write(json.dumps(data, indent=2, ensure_ascii=False))
            # os.replace is fast, but we can run it in a thread to be safe if desired.
            # For local filesystems, it's usually negligible, but let's be thorough.
            await asyncio.to_thread(os.replace, tmp_path, self.path)
        except Exception as e:
            if tmp_path.exists():
                # unlink is also sync, but usually fast.
                await asyncio.to_thread(tmp_path.unlink, missing_ok=True)
            raise e

    def exists(self) -> bool:
        return self.path.exists()

    def delete(self):
        if self.path.exists():
            self.path.unlink()
