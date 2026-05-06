from .litellm_provider import LiteLLMProvider

class OpenAIProvider(LiteLLMProvider):
    def __init__(self, api_key: str):
        super().__init__(api_key, "openai")

class AnthropicProvider(LiteLLMProvider):
    def __init__(self, api_key: str):
        super().__init__(api_key, "anthropic")

class GoogleProvider(LiteLLMProvider):
    def __init__(self, api_key: str):
        super().__init__(api_key, "google")

class OpenRouterProvider(LiteLLMProvider):
    def __init__(self, api_key: str):
        super().__init__(api_key, "openrouter")

class DeepSeekProvider(LiteLLMProvider):
    def __init__(self, api_key: str):
        super().__init__(api_key, "deepseek")

class OpenCodeGoProvider(LiteLLMProvider):
    def __init__(self, api_key: str):
        # Assuming OpenCode uses OpenRouter or similar
        super().__init__(api_key, "openrouter")

class OpenCodeZenProvider(LiteLLMProvider):
    def __init__(self, api_key: str):
        super().__init__(api_key, "openrouter")

PROVIDER_MAP = {
    "openrouter": OpenRouterProvider,
    "openai": OpenAIProvider,
    "anthropic": AnthropicProvider,
    "google": GoogleProvider,
    "deepseek": DeepSeekProvider,
    "opencode-go": OpenCodeGoProvider,
    "opencode": OpenCodeZenProvider,
}
