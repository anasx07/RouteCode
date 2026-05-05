from typing import Dict, List, Any, Optional
import httpx
from .openai import OpenAIProvider

class DeepSeekProvider(OpenAIProvider):
    def __init__(self, api_key: str):
        super().__init__(api_key)
        self.base_url = "https://api.deepseek.com/v1/chat/completions"
        self.models_url = "https://api.deepseek.com/v1/models"

    def get_models(self) -> List[Dict[str, Any]]:
        headers = {"Authorization": f"Bearer {self.api_key}"}
        try:
            response = httpx.get(self.models_url, headers=headers, timeout=10.0)
            if response.status_code == 200:
                return response.json().get("data", [])
            return []
        except Exception:
            return []
