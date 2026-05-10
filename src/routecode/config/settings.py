import os
from pathlib import Path
from typing import Dict, Optional, Any


CONFIG_DIR = Path.home() / ".routecode"
CONFIG_FILE = CONFIG_DIR / "config.json"


def _resolve_env(env_var: str, file_value: Any, default: Any) -> Any:
    """Explicit tiered precedence: env var > file > default."""
    env_val = os.environ.get(env_var)
    if env_val is not None:
        return env_val
    if file_value is not None:
        return file_value
    return default


class Config:
    """
    Application configuration with explicit tiered precedence:
      1. Environment variables (highest)
      2. Persistent JSON (~/.routecode/config.json)
      3. Hardcoded defaults (lowest)
    """

    def __init__(self):
        from ..utils.storage import AtomicJsonStore

        self.store = AtomicJsonStore(CONFIG_FILE)
        self.api_keys: Dict[str, str] = {}

        # Defaults (lowest tier)
        self._provider: str = "openrouter"
        self._model: str = "anthropic/claude-3.5-sonnet"
        self.personality: str = "default"
        self.theme: str = "lava"
        self.allowlist: list = []
        self.denylist: list = []
        self.recent_models: list = []
        self.favorites: list = []
        self.disabled_skills: list = []
        self.last_update_check: float = 0.0

        # Load file (middle tier) then env (highest tier)
        self._load()
        self._load_env_keys()

    @property
    def provider(self) -> str:
        return self._provider

    @provider.setter
    def provider(self, value: str):
        if self._provider != value:
            self._provider = value
            from ..core.events import bus

            bus.emit("config.provider_changed", provider=value)

    @property
    def model(self) -> str:
        return self._model

    @model.setter
    def model(self, value: str):
        if self._model != value:
            self._model = value
            self.add_recent_model(self._provider, value)

    def _load(self):
        """Loads config from JSON file, applying env var overrides."""
        data = self.store.load()
        if not data:
            return

        self._provider = _resolve_env(
            "ROUTECODE_PROVIDER", data.get("provider"), self._provider
        )
        self._model = _resolve_env(
            "ROUTECODE_MODEL", data.get("model"), self._model
        )
        self.personality = _resolve_env(
            "ROUTECODE_PERSONALITY", data.get("personality"), self.personality
        )
        self.theme = _resolve_env(
            "ROUTECODE_THEME", data.get("theme"), self.theme
        )
        self.allowlist = data.get("allowlist", self.allowlist)
        self.denylist = data.get("denylist", self.denylist)
        self.recent_models = data.get("recent_models", self.recent_models)
        self.favorites = data.get("favorites", self.favorites)
        self.disabled_skills = data.get("disabled_skills", self.disabled_skills)
        self.last_update_check = data.get("last_update_check", self.last_update_check)

        # File API keys — env vars merged on top in _load_env_keys()
        self.api_keys = data.get("api_keys", {})

    def _load_env_keys(self):
        """Merges environment-variable API keys into api_keys dict."""
        from ..agents.registry import PROVIDER_MAP

        for provider in PROVIDER_MAP.keys():
            key = os.environ.get(f"ROUTECODE_{provider.upper()}_KEY")
            if key:
                self.api_keys[provider] = key
        opencode_key = os.environ.get("OPENCODE_API_KEY")
        if opencode_key:
            if "opencode" not in self.api_keys:
                self.api_keys["opencode"] = opencode_key
            if "opencode-go" not in self.api_keys:
                self.api_keys["opencode-go"] = opencode_key

    def reload(self):
        """Re-reads config from disk and re-applies env overrides.
        Values that differ from file are overwritten; no events are emitted
        (handlers would not be ready during initial load)."""
        self._load()

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
            "disabled_skills": self.disabled_skills,
            "last_update_check": self.last_update_check,
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
            from ..core.events import bus

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
        self.recent_models = self.recent_models[:10]
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
