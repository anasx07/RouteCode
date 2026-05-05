import json
import time
import re
from pathlib import Path
from typing import Dict, List, Optional
from .config import CONFIG_DIR


MEMORY_DIR = CONFIG_DIR / "memory"
MEMORY_INDEX = MEMORY_DIR / "index.json"
MAX_MEMORIES = 50
MAX_MEMORY_CHARS = 500


class MemoryManager:
    def __init__(self):
        self._memories: Dict[str, str] = {}
        self._load()

    def _load(self):
        if MEMORY_INDEX.exists():
            try:
                self._memories = json.loads(MEMORY_INDEX.read_text(encoding="utf-8"))
            except Exception:
                self._memories = {}

    def _save(self):
        MEMORY_DIR.mkdir(parents=True, exist_ok=True)
        MEMORY_INDEX.write_text(json.dumps(self._memories, indent=2), encoding="utf-8")

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


memory_manager = MemoryManager()
