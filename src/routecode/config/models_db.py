import json
from typing import Dict, Optional
from functools import lru_cache
import litellm

from ..utils.paths import get_resource_path

_DB_PATH = get_resource_path("models_api.json")
_db_cache: Optional[Dict] = None


def _load_db() -> Dict:
    global _db_cache
    if _db_cache is None:
        try:
            # We only load this large JSON if litellm doesn't have the info
            # or if we need metadata specifically stored here.
            with open(_DB_PATH, "r", encoding="utf-8") as f:
                _db_cache = json.load(f)
        except Exception:
            _db_cache = {}
    return _db_cache


def get_provider(provider_id: str) -> Optional[Dict]:
    return _load_db().get(provider_id)


def get_model(provider_id: str, model_id: str) -> Optional[Dict]:
    provider = get_provider(provider_id)
    if not provider:
        return None
    models = provider.get("models", {})
    if model_id in models:
        return models[model_id]
    for mid, m in models.items():
        if mid.lower() in model_id.lower() or model_id.lower() in mid.lower():
            return m
    return None


@lru_cache(maxsize=512)
def get_model_pricing(model_id: str, provider_id: Optional[str] = None) -> tuple:
    DEFAULT_INPUT = 2.00
    DEFAULT_OUTPUT = 10.00
    DEFAULT_CONTEXT = 32000

    # 1. Try LiteLLM first (highly optimized, no large file load)
    try:
        if model_id in litellm.model_cost:
            info = litellm.model_cost[model_id]
            return (
                info.get("input_cost_per_token", 0) * 1_000_000,
                info.get("output_cost_per_token", 0) * 1_000_000,
                info.get("max_tokens", DEFAULT_CONTEXT),
            )
    except Exception:
        pass

    # 2. Try specific provider lookup in our DB (triggers load)
    if provider_id:
        model = get_model(provider_id, model_id)
        if model:
            cost = model.get("cost", {})
            limit = model.get("limit", {})
            return (
                cost.get("input", DEFAULT_INPUT),
                cost.get("output", DEFAULT_OUTPUT),
                limit.get("context", DEFAULT_CONTEXT),
            )

    # 3. Full DB scan (triggers load)
    db = _load_db()
    for pid in db:
        model = get_model(pid, model_id)
        if model:
            cost = model.get("cost", {})
            limit = model.get("limit", {})
            return (
                cost.get("input", DEFAULT_INPUT),
                cost.get("output", DEFAULT_OUTPUT),
                limit.get("context", DEFAULT_CONTEXT),
            )

    return (DEFAULT_INPUT, DEFAULT_OUTPUT, DEFAULT_CONTEXT)


def get_provider_api(provider_id: str) -> Optional[str]:
    provider = get_provider(provider_id)
    if not provider:
        return None
    return provider.get("api")


def get_provider_models(provider_id: str) -> Dict[str, Dict]:
    provider = get_provider(provider_id)
    if not provider:
        return {}
    return provider.get("models", {})


def search_models(query: str) -> list:
    """Fuzzy search for models by name or ID across all providers."""
    db = _load_db()
    results = []
    query_lower = query.lower()
    for pid, provider in db.items():
        for mid, model in provider.get("models", {}).items():
            name = model.get("name", "").lower()
            if query_lower in mid.lower() or query_lower in name:
                results.append(
                    {
                        "provider": pid,
                        "provider_name": provider.get("name", pid),
                        "model": mid,
                        "model_name": model.get("name", mid),
                        "cost": model.get("cost", {}),
                        "limit": model.get("limit", {}),
                    }
                )
    return results[:20]


@lru_cache(maxsize=256)
def get_context_limit(model_id: str, provider_id: str = "opencode") -> int:
    _, _, limit = get_model_pricing(model_id, provider_id)
    return limit
