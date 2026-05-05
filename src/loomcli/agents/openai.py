import json
import httpx
from typing import Generator, List, Dict, Any, Optional
from .base import AIProvider

def _try_parse_tool_call(tc: dict) -> Optional[dict]:
    try:
        json.loads(tc["function"]["arguments"])
        return tc
    except (json.JSONDecodeError, KeyError):
        return None


class OpenAIProvider(AIProvider):
    def __init__(self, api_key: str):
        self.api_key = api_key
        self.base_url = "https://api.openai.com/v1/chat/completions"
        self.models_url = "https://api.openai.com/v1/models"

    def get_models(self) -> List[Dict[str, Any]]:
        headers = {"Authorization": f"Bearer {self.api_key}"}
        try:
            response = httpx.get(self.models_url, headers=headers, timeout=10.0)
            if response.status_code == 200:
                return response.json().get("data", [])
            return []
        except Exception:
            return []

    def _get_headers(self) -> Dict[str, str]:
        return {
            "Authorization": f"Bearer {self.api_key}",
            "Content-Type": "application/json"
        }

    def ask(self, messages: List[Dict[str, str]], model: str, stream: bool = True, tools: Optional[List[Dict[str, Any]]] = None) -> Generator[Dict[str, Any], None, None]:
        headers = self._get_headers()
        payload = {
            "model": model,
            "messages": messages,
            "stream": stream
        }
        if tools:
            payload["tools"] = tools

        with httpx.stream("POST", self.base_url, headers=headers, json=payload, timeout=60.0) as response:
            if response.status_code != 200:
                error_body = response.read().decode()
                yield {"type": "error", "content": f"Error from OpenAI ({response.status_code}): {error_body}"}
                return

            tool_calls = {}
            yielded_indices = set()

            for line in response.iter_lines():
                if not line or line == "data: [DONE]":
                    continue

                if line.startswith("data: "):
                    try:
                        chunk = json.loads(line[6:])
                        choice = chunk.get("choices", [{}])[0]
                        delta = choice.get("delta", {})

                        finish_reason = choice.get("finish_reason")

                        content = delta.get("content", "")
                        if content:
                            yield {"type": "text", "content": content}

                        if "tool_calls" in delta:
                            for tc_delta in delta["tool_calls"]:
                                idx = tc_delta.get("index", 0)
                                if idx not in tool_calls:
                                    tool_calls[idx] = {"id": "", "type": "function", "function": {"name": "", "arguments": ""}}

                                if "id" in tc_delta:
                                    tool_calls[idx]["id"] += tc_delta["id"]
                                if "function" in tc_delta:
                                    f_delta = tc_delta["function"]
                                    if "name" in f_delta:
                                        tool_calls[idx]["function"]["name"] += f_delta["name"]
                                    if "arguments" in f_delta:
                                        tool_calls[idx]["function"]["arguments"] += f_delta["arguments"]

                            for idx, tc in tool_calls.items():
                                if idx not in yielded_indices and tc["id"] and tc["function"]["name"]:
                                    parsed = _try_parse_tool_call(tc)
                                    if parsed:
                                        yielded_indices.add(idx)
                                        yield {"type": "tool_call", "tool_call": parsed}

                        if finish_reason == "tool_calls":
                            for idx, tc in tool_calls.items():
                                if idx not in yielded_indices:
                                    parsed = _try_parse_tool_call(tc)
                                    if parsed:
                                        yield {"type": "tool_call", "tool_call": parsed}
                                    else:
                                        yield {"type": "tool_call", "tool_call": tc}
                            return

                    except json.JSONDecodeError:
                        continue

            for idx, tc in tool_calls.items():
                if idx not in yielded_indices:
                    yield {"type": "tool_call", "tool_call": tc}
