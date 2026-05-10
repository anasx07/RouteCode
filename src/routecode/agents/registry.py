import json
from typing import Dict, Any
from .litellm_provider import LiteLLMProvider
from .cloudflare_provider import CloudflareProvider
from ..utils.paths import get_resource_path
from .mapping import normalize_provider_name

MODELS_API_PATH = get_resource_path("models_api.json")


def _load_registry() -> Dict[str, Any]:
    if MODELS_API_PATH.exists():
        try:
            with open(MODELS_API_PATH, "r", encoding="utf-8") as f:
                return json.load(f)
        except Exception:
            return {}
    return {}


REGISTRY_DATA = _load_registry()


def get_provider_class(provider_id: str):
    """
    Returns a provider class factory that selects the best implementation
    (Native or LiteLLM) based on metadata.
    """
    data = REGISTRY_DATA.get(provider_id, {})
    npm = data.get("npm", "")
    base_url = data.get("api")

    pid_lower = provider_id.lower()
    is_native_cloudflare = ("workers-ai" in pid_lower) or (
        "cloudflare" in pid_lower and "gateway" not in pid_lower
    )
    litellm_p = normalize_provider_name(npm) or provider_id

    class DynamicProvider:
        def __init__(self, api_key: str):
            models_dict = data.get("models", {})
            models_list = [
                {"id": mid, "name": m.get("name", mid)}
                for mid, m in models_dict.items()
            ]

            if is_native_cloudflare:
                self.impl = CloudflareProvider(
                    api_key, base_url=base_url, models=models_list
                )
            else:
                self.impl = LiteLLMProvider(
                    api_key, litellm_p, base_url=base_url, models=models_list
                )

        async def ask(self, *args, **kwargs):
            async for chunk in self.impl.ask(*args, **kwargs):
                yield chunk

        async def get_models(self):
            return await self.impl.get_models()

    return DynamicProvider


PROVIDER_MAP = {pid: get_provider_class(pid) for pid in REGISTRY_DATA.keys()}

if not PROVIDER_MAP:

    class OpenAIProvider(LiteLLMProvider):
        def __init__(self, api_key: str):
            super().__init__(api_key, "openai")

    class AnthropicProvider(LiteLLMProvider):
        def __init__(self, api_key: str):
            super().__init__(api_key, "anthropic")

    PROVIDER_MAP = {
        "openai": OpenAIProvider,
        "anthropic": AnthropicProvider,
    }
