import os
from pathlib import Path
from typing import Dict, Optional, Any

CONFIG_DIR = Path.home() / ".loomcli"
CONFIG_FILE = CONFIG_DIR / "config.json"


class Config:
    def __init__(self):
        from .core.storage import AtomicJsonStore

        self._provider: str = os.environ.get("LOOM_PROVIDER", "openrouter")
        self._model: str = os.environ.get("LOOM_MODEL", "anthropic/claude-3.5-sonnet")
        self.personality: str = os.environ.get("LOOM_PERSONALITY", "default")
        self.theme: str = os.environ.get("LOOM_THEME", "lava")
        self.allowlist: list = []
        self.denylist: list = []
        self.api_keys: Dict[str, str] = {}
        self.recent_models: list = []  # List of (provider, model) tuples
        self.favorites: list = []  # List of (provider, model) tuples
        self.store = AtomicJsonStore(CONFIG_FILE)
        self._load()
        self._load_env_keys()

    @property
    def provider(self) -> str:
        return self._provider

    @provider.setter
    def provider(self, value: str):
        if self._provider != value:
            self._provider = value
            from .core import bus

            bus.emit("config.provider_changed", provider=value)

    @property
    def model(self) -> str:
        return self._model

    @model.setter
    def model(self, value: str):
        if self._model != value:
            self._model = value
            self.add_recent_model(self._provider, value)
            from .core import bus

            bus.emit("config.model_changed", model=value)

    def _load(self):
        data = self.store.load()
        if data:
            if not os.environ.get("LOOM_PROVIDER"):
                self._provider = data.get("provider", self._provider)
            if not os.environ.get("LOOM_MODEL"):
                self._model = data.get("model", self._model)
            if not os.environ.get("LOOM_PERSONALITY"):
                self.personality = data.get("personality", self.personality)
            if not os.environ.get("LOOM_THEME"):
                self.theme = data.get("theme", self.theme)
            self.allowlist = data.get("allowlist", [])
            self.denylist = data.get("denylist", [])
            self.api_keys = data.get("api_keys", {})
            self.recent_models = data.get("recent_models", [])
            self.favorites = data.get("favorites", [])

    def _load_env_keys(self):
        from .agents.registry import PROVIDER_MAP

        for provider in PROVIDER_MAP.keys():
            key = os.environ.get(f"LOOM_{provider.upper()}_KEY")
            if key:
                self.api_keys[provider] = key
        opencode_key = os.environ.get("OPENCODE_API_KEY")
        if opencode_key:
            if "opencode" not in self.api_keys:
                self.api_keys["opencode"] = opencode_key
            if "opencode-go" not in self.api_keys:
                self.api_keys["opencode-go"] = opencode_key

    def to_dict(self) -> Dict[str, Any]:
        """Returns a dictionary representation of the current configuration."""
        return {
            "provider": self.provider,
            "model": self.model,
            "personality": self.personality,
            "theme": self.theme,
            "api_keys": self.api_keys,
            "allowlist": self.allowlist,
            "denylist": self.denylist,
            "recent_models": self.recent_models,
            "favorites": self.favorites,
        }

    def save(self):
        """Saves configuration synchronously."""
        self.store.save(self.to_dict())

    async def save_async(self):
        """Saves configuration asynchronously."""
        await self.store.save_async(self.to_dict())

    def set_api_key(self, provider: str, key: str):
        self.api_keys[provider] = key
        if provider == self.provider:
            from .core import bus

            bus.emit("config.provider_changed", provider=provider)
        self.save()

    def get_api_key(self, provider: Optional[str] = None) -> Optional[str]:
        p = provider or self.provider
        return self.api_keys.get(p)

    def add_recent_model(self, provider: str, model: str):
        item = [provider, model]
        if item in self.recent_models:
            self.recent_models.remove(item)
        self.recent_models.insert(0, item)
        self.recent_models = self.recent_models[:10]  # Keep last 10
        self.save()

    def toggle_favorite(self, provider: str, model: str):
        item = [provider, model]
        if item in self.favorites:
            self.favorites.remove(item)
        else:
            self.favorites.append(item)
        self.save()


# Global config instance
config = Config()
