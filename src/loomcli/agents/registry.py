import json
from pathlib import Path
from typing import Dict, Any
from .litellm_provider import LiteLLMProvider

MODELS_API_PATH = Path(__file__).parent.parent / "models_api.json"

def _load_registry() -> Dict[str, Any]:
    if MODELS_API_PATH.exists():
        try:
            with open(MODELS_API_PATH, "r", encoding="utf-8") as f:
                return json.load(f)
        except Exception:
            return {}
    return {}

REGISTRY_DATA = _load_registry()

class DynamicLiteLLMProvider(LiteLLMProvider):
    """
    A LiteLLMProvider that configures itself from models_api.json metadata.
    """
    def __init__(self, api_key: str, provider_id: str):
        data = REGISTRY_DATA.get(provider_id, {})
        base_url = data.get("api")
        npm = data.get("npm", "")
        
        # Determine internal LiteLLM provider name from metadata hints
        litellm_p = provider_id
        if "openai" in npm: 
            litellm_p = "openai"
        elif "anthropic" in npm: 
            litellm_p = "anthropic"
        elif "google" in npm or "gemini" in npm: 
            litellm_p = "google"
        elif "deepseek" in npm:
            litellm_p = "deepseek"
        
        # Populate models list from metadata
        models_dict = data.get("models", {})
        models_list = [{"id": mid, "name": m.get("name", mid)} for mid, m in models_dict.items()]
        
        super().__init__(api_key, litellm_p, base_url=base_url, models=models_list)

def create_provider_factory(pid: str):
    """Creates a provider class compatible with the existing PROVIDER_MAP interface."""
    class SpecificProvider(DynamicLiteLLMProvider):
        def __init__(self, api_key: str):
            super().__init__(api_key, pid)
    return SpecificProvider

# Build the map dynamically from JSON
PROVIDER_MAP = {pid: create_provider_factory(pid) for pid in REGISTRY_DATA.keys()}

# Fallback to standard providers if JSON is missing or empty
if not PROVIDER_MAP:
    class OpenAIProvider(LiteLLMProvider):
        def __init__(self, api_key: str): super().__init__(api_key, "openai")
    
    class AnthropicProvider(LiteLLMProvider):
        def __init__(self, api_key: str): super().__init__(api_key, "anthropic")
        
    class GoogleProvider(LiteLLMProvider):
        def __init__(self, api_key: str): super().__init__(api_key, "google")
        
    class OpenRouterProvider(LiteLLMProvider):
        def __init__(self, api_key: str): super().__init__(api_key, "openrouter")

    PROVIDER_MAP = {
        "openai": OpenAIProvider,
        "anthropic": AnthropicProvider,
        "google": GoogleProvider,
        "openrouter": OpenRouterProvider,
        "deepseek": lambda k: LiteLLMProvider(k, "deepseek"),
    }
