import os
import json
from pathlib import Path
from typing import Dict, Optional

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
        self._load()
        self._load_env_keys()

    def _load(self):
        if CONFIG_FILE.exists():
            try:
                with open(CONFIG_FILE, "r") as f:
                    data = json.load(f)
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
            except Exception:
                pass

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
        CONFIG_DIR.mkdir(parents=True, exist_ok=True)
        data = {
            "provider": self.provider,
            "model": self.model,
            "personality": self.personality,
            "theme": self.theme,
            "api_keys": self.api_keys
        }
        with open(CONFIG_FILE, "w") as f:
            json.dump(data, f, indent=4)

    def set_api_key(self, provider: str, key: str):
        self.api_keys[provider] = key
        self.save()

    def get_api_key(self, provider: Optional[str] = None) -> Optional[str]:
        p = provider or self.provider
        return self.api_keys.get(p)

# Global config instance
config = Config()
