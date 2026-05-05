from .openai import OpenAIProvider
from .anthropic import AnthropicProvider
from .google import GoogleProvider
from .openrouter import OpenRouterProvider
from .deepseek import DeepSeekProvider
from .opencode_go import OpenCodeGoProvider
from .opencode_zen import OpenCodeZenProvider

PROVIDER_MAP = {
    "openrouter": OpenRouterProvider,
    "openai": OpenAIProvider,
    "anthropic": AnthropicProvider,
    "google": GoogleProvider,
    "deepseek": DeepSeekProvider,
    "opencode-go": OpenCodeGoProvider,
    "opencode": OpenCodeZenProvider,
}
