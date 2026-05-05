from typing import Dict, List, Any, Optional
import httpx
from .openai import OpenAIProvider

class OpenCodeGoProvider(OpenAIProvider):
    def __init__(self, api_key: str):
        super().__init__(api_key)
        self.base_url = "https://opencode.ai/zen/go/v1/chat/completions"
        self.models_url = "https://opencode.ai/zen/go/v1/models"

    def get_models(self) -> List[Dict[str, Any]]:
        return [
            {"id": "kimi-k2.5", "name": "Kimi K2.5"},
            {"id": "minimax-m2.7", "name": "MiniMax M2.7"},
            {"id": "glm-5", "name": "GLM-5"},
            {"id": "minimax-m2.5", "name": "MiniMax M2.5"},
        ]
