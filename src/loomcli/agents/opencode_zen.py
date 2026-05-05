from typing import Dict, List, Any, Optional
import httpx
from .openai import OpenAIProvider

class OpenCodeZenProvider(OpenAIProvider):
    def __init__(self, api_key: str):
        super().__init__(api_key)
        self.base_url = "https://opencode.ai/zen/v1/chat/completions"
        self.models_url = "https://opencode.ai/zen/v1/models"

    def get_models(self) -> List[Dict[str, Any]]:
        headers = {"Authorization": f"Bearer {self.api_key}"}
        try:
            response = httpx.get(self.models_url, headers=headers, timeout=10.0)
            if response.status_code == 200:
                return response.json().get("data", [])
            return self._get_fallback_models()
        except Exception:
            return self._get_fallback_models()

    def _get_fallback_models(self) -> List[Dict[str, Any]]:
        return [
            {"id": "gpt-5.2-codex", "name": "GPT-5.2 Codex"},
            {"id": "claude-sonnet-4-5", "name": "Claude Sonnet 4.5"},
            {"id": "gemini-3-pro", "name": "Gemini 3 Pro"},
            {"id": "mimo-v2-flash-free", "name": "Mimo v2 Flash (Free)"},
        ]
