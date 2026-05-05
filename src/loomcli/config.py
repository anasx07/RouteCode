import os
import json
from pathlib import Path
from typing import Dict, Optional

from .storage import AtomicJsonStore

CONFIG_DIR = Path.home() / ".loomcli"
CONFIG_FILE = CONFIG_DIR / "config.json"

class Config:
    def __init__(self):
        self.provider: str = os.environ.get("LOOM_PROVIDER", "openrouter")
        self.model: str = os.environ.get("LOOM_MODEL", "anthropic/claude-3.5-sonnet")
        self.personality: str = os.environ.get("LOOM_PERSONALITY", "default")
        self.theme: str = os.environ.get("LOOM_THEME", "lava")
        self.allowlist: list = []
        self.denylist: list = []
        self.api_keys: Dict[str, str] = {}
        self.store = AtomicJsonStore(CONFIG_FILE)
        self._load()
        self._load_env_keys()

    def _load(self):
        data = self.store.load()
        if data:
            if not os.environ.get("LOOM_PROVIDER"):
                self.provider = data.get("provider", self.provider)
            if not os.environ.get("LOOM_MODEL"):
                self.model = data.get("model", self.model)
            if not os.environ.get("LOOM_PERSONALITY"):
                self.personality = data.get("personality", self.personality)
            if not os.environ.get("LOOM_THEME"):
                self.theme = data.get("theme", self.theme)
            self.allowlist = data.get("allowlist", [])
            self.denylist = data.get("denylist", [])
            self.api_keys = data.get("api_keys", {})

    def _load_env_keys(self):
        for provider in ("openrouter", "openai", "anthropic", "google", "deepseek", "opencode", "opencode-go"):
            key = os.environ.get(f"LOOM_{provider.upper()}_KEY")
            if key:
                self.api_keys[provider] = key
        opencode_key = os.environ.get("OPENCODE_API_KEY")
        if opencode_key:
            if "opencode" not in self.api_keys:
                self.api_keys["opencode"] = opencode_key
            if "opencode-go" not in self.api_keys:
                self.api_keys["opencode-go"] = opencode_key

    def save(self):
        data = {
            "provider": self.provider,
            "model": self.model,
            "personality": self.personality,
            "theme": self.theme,
            "api_keys": self.api_keys,
            "allowlist": self.allowlist,
            "denylist": self.denylist
        }
        self.store.save(data)

    async def save_async(self):
        data = {
            "provider": self.provider,
            "model": self.model,
            "personality": self.personality,
            "theme": self.theme,
            "api_keys": self.api_keys,
            "allowlist": self.allowlist,
            "denylist": self.denylist
        }
        await self.store.save_async(data)

    def set_api_key(self, provider: str, key: str):
        self.api_keys[provider] = key
        self.save()

    def get_api_key(self, provider: Optional[str] = None) -> Optional[str]:
        p = provider or self.provider
        return self.api_keys.get(p)

# Global config instance
config = Config()
