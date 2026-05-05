import json
import httpx
from typing import Generator, List, Dict, Any, Optional
from .base import AIProvider


class AnthropicProvider(AIProvider):
    def __init__(self, api_key: str):
        self.api_key = api_key
        self.base_url = "https://api.anthropic.com/v1/messages"
        self.api_version = "2023-06-01"

    def _convert_messages(self, messages: List[Dict]) -> tuple:
        system = None
        converted = []
        for msg in messages:
            role = msg["role"]
            if role == "system":
                system = msg["content"]
                continue
            content = msg.get("content") or ""
            if role == "tool":
                converted.append({
                    "role": "user",
                    "content": [{
                        "type": "tool_result",
                        "tool_use_id": msg.get("tool_call_id", ""),
                        "content": msg.get("content", "")
                    }]
                })
            elif role == "assistant":
                content_parts = []
                if content:
                    content_parts.append({"type": "text", "text": content})
                for tc in msg.get("tool_calls", []):
                    try:
                        args = json.loads(tc["function"]["arguments"]) if isinstance(tc["function"]["arguments"], str) else tc["function"]["arguments"]
                    except Exception:
                        args = tc["function"]["arguments"]
                    content_parts.append({
                        "type": "tool_use",
                        "id": tc["id"],
                        "name": tc["function"]["name"],
                        "input": args
                    })
                converted.append({"role": "assistant", "content": content_parts})
            else:
                converted.append({"role": role, "content": content})
        return system, converted

    def _convert_tools(self, tools: List[Dict]) -> List[Dict]:
        result = []
        for t in tools:
            fn = t.get("function", {})
            result.append({
                "name": fn.get("name", ""),
                "description": fn.get("description", ""),
                "input_schema": fn.get("parameters", {})
            })
        return result

    def ask(self, messages: List[Dict[str, str]], model: str, stream: bool = True, tools: Optional[List[Dict[str, Any]]] = None) -> Generator[Dict[str, Any], None, None]:
        system, converted = self._convert_messages(messages)

        payload = {
            "model": model,
            "messages": converted,
            "max_tokens": 4096,
            "stream": stream
        }
        if system:
            payload["system"] = system
        if tools:
            payload["tools"] = self._convert_tools(tools)

        headers = {
            "x-api-key": self.api_key,
            "anthropic-version": self.api_version,
            "Content-Type": "application/json"
        }

        with httpx.stream("POST", self.base_url, headers=headers, json=payload, timeout=60.0) as response:
            if response.status_code != 200:
                error_body = response.read().decode()
                yield {"type": "error", "content": f"Error from Anthropic ({response.status_code}): {error_body}"}
                return

            current_tool_block = None

            for line in response.iter_lines():
                if not line:
                    continue
                if line.startswith("event: "):
                    continue
                if not line.startswith("data: "):
                    continue

                data = line[6:]
                if data == "[DONE]":
                    break

                try:
                    event = json.loads(data)
                except json.JSONDecodeError:
                    continue

                event_type = event.get("type", "")

                if event_type == "content_block_start":
                    block = event.get("content_block", {})
                    if block.get("type") == "tool_use":
                        current_tool_block = {
                            "id": block.get("id", ""),
                            "type": "function",
                            "function": {
                                "name": block.get("name", ""),
                                "arguments": ""
                            }
                        }

                elif event_type == "content_block_delta":
                    delta = event.get("delta", {})
                    if delta.get("type") == "text_delta":
                        yield {"type": "text", "content": delta.get("text", "")}
                    elif delta.get("type") == "input_json_delta":
                        if current_tool_block:
                            current_tool_block["function"]["arguments"] += delta.get("partial_json", "")

                elif event_type == "content_block_stop":
                    if current_tool_block:
                        yield {"type": "tool_call", "tool_call": current_tool_block}
                        current_tool_block = None

            if current_tool_block:
                yield {"type": "tool_call", "tool_call": current_tool_block}
