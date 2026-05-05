import json
import httpx
from typing import Generator, List, Dict, Any, Optional
from .base import AIProvider


class GoogleProvider(AIProvider):
    def __init__(self, api_key: str):
        self.api_key = api_key
        self.base_url = "https://generativelanguage.googleapis.com/v1beta/models"

    def _convert_messages(self, messages: List[Dict]) -> List[Dict]:
        contents = []
        system = None
        for msg in messages:
            role = msg["role"]
            if role == "system":
                system = msg.get("content", "")
                continue
            gemini_role = "model" if role == "assistant" else "user"
            content = msg.get("content") or ""
            parts = [{"text": content}] if content else []

            for tc in msg.get("tool_calls", []):
                try:
                    args = json.loads(tc["function"]["arguments"]) if isinstance(tc["function"]["arguments"], str) else tc["function"]["arguments"]
                except Exception:
                    args = tc["function"]["arguments"]
                parts.append({
                    "functionCall": {
                        "name": tc["function"]["name"],
                        "args": args
                    }
                })

            if role == "tool":
                parts.append({
                    "functionResponse": {
                        "name": msg.get("name", ""),
                        "response": {
                            "name": msg.get("name", ""),
                            "content": msg.get("content", "{}")
                        }
                    }
                })

            if parts:
                contents.append({"role": gemini_role, "parts": parts})

        return contents, system

    def _convert_tools(self, tools: List[Dict]) -> List[Dict]:
        result = []
        for t in tools:
            fn = t.get("function", {})
            result.append({
                "functionDeclarations": [{
                    "name": fn.get("name", ""),
                    "description": fn.get("description", ""),
                    "parameters": fn.get("parameters", {})
                }]
            })
        return result

    def ask(self, messages: List[Dict[str, str]], model: str, stream: bool = True, tools: Optional[List[Dict[str, Any]]] = None) -> Generator[Dict[str, Any], None, None]:
        contents, system = self._convert_messages(messages)

        generation_config = {"temperature": 0.7, "maxOutputTokens": 4096}
        payload = {"contents": contents}
        if system:
            payload["systemInstruction"] = {"parts": [{"text": system}]}
        payload["generationConfig"] = generation_config
        if tools:
            payload["tools"] = self._convert_tools(tools)

        url = f"{self.base_url}/{model}:streamGenerateContent?alt=sse&key={self.api_key}"
        headers = {"Content-Type": "application/json"}

        with httpx.stream("POST", url, headers=headers, json=payload, timeout=60.0) as response:
            if response.status_code != 200:
                error_body = response.read().decode()
                yield {"type": "error", "content": f"Error from Google ({response.status_code}): {error_body}"}
                return

            for line in response.iter_lines():
                if not line:
                    continue
                if not line.startswith("data: "):
                    continue
                data = line[6:]
                if data == "[DONE]":
                    break

                try:
                    chunk = json.loads(data)
                except json.JSONDecodeError:
                    continue

                candidates = chunk.get("candidates", [])
                if not candidates:
                    continue

                content = candidates[0].get("content", {})
                parts = content.get("parts", [])

                for part in parts:
                    if "text" in part:
                        yield {"type": "text", "content": part["text"]}
                    elif "functionCall" in part:
                        fc = part["functionCall"]
                        yield {
                            "type": "tool_call",
                            "tool_call": {
                                "id": fc.get("name", "unknown"),
                                "type": "function",
                                "function": {
                                    "name": fc.get("name", ""),
                                    "arguments": fc.get("args", {})
                                }
                            }
                        }
