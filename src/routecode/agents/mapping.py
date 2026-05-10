"""
Single source of truth for provider-to-LiteLLM name mapping and model prefix logic.
"""

PROVIDER_TO_PREFIX = {
    "openai": "openai/",
    "anthropic": "anthropic/",
    "google": "gemini/",
    "gemini": "gemini/",
    "deepseek": "deepseek/",
    "openrouter": "openrouter/",
    "vertex_ai": "vertex_ai/",
    "groq": "groq/",
    "mistral": "mistral/",
    "cloudflare": "openai/",
}

LITELLM_PREFIXES = sorted(set(PROVIDER_TO_PREFIX.values()))


def get_model_prefix(provider_name: str) -> str:
    """Returns the LiteLLM model prefix for a provider (e.g. 'openai/', 'gemini/')."""
    return PROVIDER_TO_PREFIX.get(provider_name, f"{provider_name}/")


def resolve_model_name(provider_name: str, model: str) -> str:
    """Prepends the LiteLLM prefix to model if not already present."""
    prefix = get_model_prefix(provider_name)
    if model.startswith(tuple(LITELLM_PREFIXES)):
        return model
    return f"{prefix}{model}"


def get_model_list_pattern(provider_name: str) -> str:
    """Returns the LiteLLM model list pattern for a provider."""
    return get_model_prefix(provider_name) + "*"


def normalize_provider_name(npm: str) -> str:
    """Maps an npm-style metadata string to a canonical provider name."""
    npm_lower = npm.lower()
    if "openai" in npm_lower:
        return "openai"
    if "anthropic" in npm_lower:
        return "anthropic"
    if "google" in npm_lower or "gemini" in npm_lower:
        return "google"
    if "deepseek" in npm_lower:
        return "deepseek"
    if "workers-ai" in npm_lower or ("cloudflare" in npm_lower and "gateway" not in npm_lower):
        return "cloudflare"
    return npm_lower
