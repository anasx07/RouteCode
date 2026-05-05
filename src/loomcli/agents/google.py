import json
from typing import List, Dict, Any, Optional, Generator
from .protocol import LoomMessage, MessageAdapter
from .base import AIProvider

class GoogleAdapter(MessageAdapter):
    def to_provider(self, messages: List[LoomMessage]) -> Any:
        contents = []
        system = None
        for msg in messages:
            role = msg.role
            if role == "system":
                system = msg.content if isinstance(msg.content, str) else ""
                continue
            
            gemini_role = "model" if role == "assistant" else "user"
            content = msg.content or ""
            parts = [{"text": str(content)}] if content else []

            if msg.tool_calls:
                for tc in msg.tool_calls:
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
                        "name": msg.name or "",
                        "response": {
                            "name": msg.name or "",
                            "content": str(msg.content or "{}")
                        }
                    }
                })

            if parts:
                contents.append({"role": gemini_role, "parts": parts})

        return system, contents

    def to_provider_tools(self, tools: List[Dict]) -> List[Dict]:
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


class GoogleProvider(AIProvider):
    def __init__(self, api_key: str):
        super().__init__(api_key)
        self.base_url = "https://generativelanguage.googleapis.com/v1beta/models"
        self.adapter = GoogleAdapter()

    def ask(self, messages: List[Dict[str, Any]], model: str, stream: bool = True, tools: Optional[List[Dict[str, Any]]] = None) -> Generator[Dict[str, Any], None, None]:
        # Convert dict messages to LoomMessage objects
        loom_messages = [LoomMessage(**m) for m in messages]
        system, contents = self.adapter.to_provider(loom_messages)

        generation_config = {"temperature": 0.7, "maxOutputTokens": 4096}
        payload = {"contents": contents}
        if system:
            payload["systemInstruction"] = {"parts": [{"text": system}]}
        payload["generationConfig"] = generation_config
        if tools:
            payload["tools"] = self.adapter.to_provider_tools(tools)

        url = f"{self.base_url}/{model}:streamGenerateContent?alt=sse&key={self.api_key}"
        headers = {"Content-Type": "application/json"}

        for data in self.transport.stream_post(url, headers, payload):
            try:
                chunk = json.loads(data)
                
                # Check for transport errors (No longer needed, exceptions bubble up)

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
            except json.JSONDecodeError:
                continue
