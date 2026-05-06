import json
import time
import re
from pathlib import Path
from typing import Dict, List, Optional
from .core.storage import AtomicJsonStore
from .config import CONFIG_DIR


MAX_MEMORIES = 50
MAX_MEMORY_CHARS = 500


class MemoryManager:
    def __init__(self, config_dir: Path):
        self.config_dir = config_dir
        self.memory_dir = config_dir / "memory"
        self.memory_index = self.memory_dir / "index.json"
        
        # Ensure directory exists
        self.memory_dir.mkdir(parents=True, exist_ok=True)
        
        self._memories: Dict[str, str] = {}
        self.store = AtomicJsonStore(self.memory_index)
        # self._load() is now called asynchronously via _load_async in the REPL

    def _load(self):
        self._memories = self.store.load()

    async def _load_async(self):
        self._memories = await self.store.load_async()

    def _save(self):
        self.store.save(self._memories)

    async def _save_async(self):
        await self.store.save_async(self._memories)

    def remember(self, key: str, value: str) -> str:
        key = key.strip().lower().replace(" ", "_")[:40]
        value = value.strip()[:MAX_MEMORY_CHARS]
        if not key or not value:
            return "Key and value cannot be empty."

        self._memories[key] = value

        # Enforce max memory count
        if len(self._memories) > MAX_MEMORIES:
            oldest = sorted(self._memories.items(), key=lambda x: x[0])[:len(self._memories) - MAX_MEMORIES]
            for k, _ in oldest:
                del self._memories[k]

        self._save()
        return f"Remembered: {key}"

    def forget(self, key: str) -> str:
        key = key.strip().lower().replace(" ", "_")[:40]
        if key in self._memories:
            del self._memories[key]
            self._save()
            return f"Forgot: {key}"
        return f"No memory found for: {key}"

    def get(self, key: str) -> Optional[str]:
        return self._memories.get(key.strip().lower().replace(" ", "_")[:40])

    def list(self) -> Dict[str, str]:
        return dict(sorted(self._memories.items()))

    def get_relevant_context(self, query: str = "") -> str:
        if not self._memories:
            return ""

        if query:
            query_lower = query.lower()
            terms = re.findall(r'\w+', query_lower)
            scored = []
            for key, value in self._memories.items():
                score = sum(1 for t in terms if t in key.lower() or t in value.lower())
                if score > 0:
                    scored.append((score, key, value))
            scored.sort(reverse=True)
            relevant = scored[:5]
        else:
            relevant = [(0, k, v) for k, v in self._memories.items()]

        if relevant:
            lines = ["## Session Memory"]
            for _, key, value in relevant:
                lines.append(f"- {key}: {value}")
            return "\n".join(lines)
        return ""


