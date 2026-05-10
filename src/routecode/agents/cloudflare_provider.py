import json
import httpx
import os
from typing import List, Dict, Any, Optional, AsyncGenerator
from .base import AIProvider


class CloudflareProvider(AIProvider):
    """
    Native Cloudflare Workers AI provider bypassing LiteLLM to avoid Pydantic issues.
    """

    def __init__(
        self,
        api_key: str,
        account_id: Optional[str] = None,
        base_url: Optional[str] = None,
        models: Optional[List[Dict[str, Any]]] = None,
    ):
        super().__init__(api_key)
        self.account_id = account_id or os.environ.get("CLOUDFLARE_ACCOUNT_ID")
        self.base_url = base_url
        self.models_list = models

        # Unpack JSON key if needed
        if api_key.startswith("{") and api_key.endswith("}"):
            try:
                data = json.loads(api_key)
                self.account_id = data.get("CLOUDFLARE_ACCOUNT_ID", self.account_id)
                self.api_key = data.get("CLOUDFLARE_API_KEY", self.api_key)
            except Exception:
                pass

    async def ask(
        self,
        messages: List[Dict[str, Any]],
        model: str,
        stream: bool = True,
        tools: Optional[List[Dict[str, Any]]] = None,
    ) -> AsyncGenerator[Dict[str, Any], None]:
        # Sanitize messages for Cloudflare (remove tool calls/results if present)
        sanitized_messages = []
        for m in messages:
            if m.get("role") in ["user", "system", "assistant"]:
                msg = {"role": m["role"], "content": m.get("content") or ""}
                sanitized_messages.append(msg)

        url = f"https://api.cloudflare.com/client/v4/accounts/{self.account_id}/ai/run/{model}"
        headers = {"Authorization": f"Bearer {self.api_key}"}

        # Cloudflare native API expects 'messages' in the body
        payload = {"messages": sanitized_messages, "stream": stream}

        from ..utils.logger import get_logger
        logger = get_logger(__name__)
        logger.debug(f"Cloudflare Request: POST {url}")

        try:
            async with httpx.AsyncClient(timeout=60.0) as client:
                if stream:
                    async with client.stream(
                        "POST", url, headers=headers, json=payload
                    ) as response:
                        if response.status_code != 200:
                            err_text = await response.aread()
                            logger.error(f"Cloudflare Error {response.status_code}: {err_text.decode()}")
                            yield {
                                "type": "error",
                                "content": f"Cloudflare error {response.status_code}: {err_text.decode()}",
                            }
                            return

                        last_text_len = 0
                        last_thought_len = 0
                        
                        async for line in response.aiter_lines():
                            if line.startswith("data:"):
                                data_str = line[len("data:"):].strip()
                                if data_str == "[DONE]":
                                    break
                                try:
                                    logger.debug(f"Raw Cloudflare Chunk: {data_str}")
                                    chunk = json.loads(data_str)
                                    
                                    # Handle OpenAI-compatible streaming format (choices -> delta)
                                    choices = chunk.get("choices", [])
                                    if choices:
                                        delta = choices[0].get("delta", {})
                                        
                                        # Handle reasoning content
                                        reasoning = delta.get("reasoning_content")
                                        if reasoning:
                                            # If reasoning is cumulative, yield only the new part
                                            if len(reasoning) > last_thought_len:
                                                yield {"type": "thought", "content": reasoning[last_thought_len:]}
                                                last_thought_len = len(reasoning)
                                            else:
                                                # If it's a delta, just yield it
                                                yield {"type": "thought", "content": reasoning}
                                        
                                        content = delta.get("content")
                                        if content:
                                            # If content is cumulative, yield only the new part
                                            if len(content) > last_text_len:
                                                yield {"type": "text", "content": content[last_text_len:]}
                                                last_text_len = len(content)
                                            else:
                                                # If it's a delta, just yield it
                                                yield {"type": "text", "content": content}
                                        continue

                                    # Fallback for older Workers AI format (often cumulative)
                                    content = chunk.get("response") or chunk.get("text") or chunk.get("result")
                                    if content is None and "result" in chunk and isinstance(chunk["result"], dict):
                                        content = chunk["result"].get("response") or chunk["result"].get("text")
                                    
                                    if content is not None:
                                        content_str = str(content)
                                        if len(content_str) > last_text_len:
                                            yield {"type": "text", "content": content_str[last_text_len:]}
                                            last_text_len = len(content_str)
                                        else:
                                            yield {"type": "text", "content": content_str}
                                except Exception as e:
                                    logger.error(f"Failed to parse Cloudflare chunk: {e} | Raw: {data_str}")
                                    continue
                else:
                    resp = await client.post(url, headers=headers, json=payload)
                    if resp.status_code == 200:
                        data = resp.json()
                        result = data.get("result", {})
                        
                        # Handle OpenAI-compatible non-streaming format
                        choices = result.get("choices", []) or data.get("choices", [])
                        if choices:
                            message = choices[0].get("message", {})
                            reasoning = message.get("reasoning_content")
                            if reasoning:
                                yield {"type": "thought", "content": reasoning}
                            content = message.get("content")
                            if content:
                                yield {"type": "text", "content": content}
                        else:
                            # Fallback for older format
                            content = result.get("response") or result.get("text") or result.get("result") or data.get("response")
                            if content:
                                yield {"type": "text", "content": content}
                    else:
                        yield {
                            "type": "error",
                            "content": f"Cloudflare error {resp.status_code}: {resp.text}",
                        }
        except Exception as e:
            yield {"type": "error", "content": str(e)}

    async def get_models(self) -> List[Dict[str, Any]]:
        if not self.account_id:
            return self.models_list or []

        url = f"https://api.cloudflare.com/client/v4/accounts/{self.account_id}/ai/models/search?task=Text%20Generation"
        headers = {"Authorization": f"Bearer {self.api_key}"}
        try:
            async with httpx.AsyncClient(timeout=10.0) as client:
                resp = await client.get(url, headers=headers)
                if resp.status_code == 200:
                    data = resp.json()
                    result_data = data.get("result", [])
                    return [
                        {"id": m["name"], "name": m["name"].split("/")[-1]}
                        for m in result_data
                        if "name" in m
                    ]
        except Exception:
            pass
        return self.models_list or []
