from typing import Dict, List, Any, Optional
import httpx
from .openai import OpenAIProvider

class OpenRouterProvider(OpenAIProvider):
    def __init__(self, api_key: str):
        super().__init__(api_key)
        self.base_url = "https://openrouter.ai/api/v1/chat/completions"
        self.models_url = "https://openrouter.ai/api/v1/models"

    def _get_headers(self) -> Dict[str, str]:
        return {
            "Authorization": f"Bearer {self.api_key}",
            "HTTP-Referer": "https://github.com/loomcli",
            "X-Title": "LoomCLI",
            "Content-Type": "application/json"
        }

    def get_models(self) -> List[Dict[str, Any]]:
        headers = {"Authorization": f"Bearer {self.api_key}"}
        try:
            response = httpx.get(self.models_url, headers=headers, timeout=10.0)
            if response.status_code == 200:
                return response.json().get("data", [])
            return []
        except Exception:
            return []
