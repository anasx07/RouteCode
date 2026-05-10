import asyncio
import litellm
from typing import List, Dict, Any, Optional, AsyncGenerator
from .base import AIProvider
from .mapping import resolve_model_name, get_model_list_pattern

litellm.set_verbose = False
litellm.suppress_debug_info = True


class LiteLLMProvider(AIProvider):
    """
    Standardized AI provider using LiteLLM for cross-provider compatibility.
    """

    def __init__(
        self,
        api_key: str,
        provider_name: str,
        base_url: Optional[str] = None,
        models: Optional[List[Dict[str, Any]]] = None,
    ):
        super().__init__(api_key)
        self.provider_name = provider_name
        self.base_url = base_url
        self.models_list = models

        if api_key.startswith("{") and api_key.endswith("}"):
            try:
                import json
                import os

                keys = json.loads(api_key)
                for k, v in keys.items():
                    os.environ[k] = v

                for k, v in keys.items():
                    if "KEY" in k or "TOKEN" in k or "SECRET" in k:
                        self.api_key = v
                        break

                if self.base_url:
                    for k, v in keys.items():
                        self.base_url = self.base_url.replace(f"${{{k}}}", v)
                        self.base_url = self.base_url.replace(f"${k}", v)
            except Exception:
                pass

    async def ask(
        self,
        messages: List[Dict[str, Any]],
        model: str,
        stream: bool = True,
        tools: Optional[List[Dict[str, Any]]] = None,
    ) -> AsyncGenerator[Dict[str, Any], None]:
        litellm_model = resolve_model_name(self.provider_name, model)

        completion_args = {
            "model": litellm_model,
            "messages": messages,
            "stream": stream,
            "api_key": self.api_key,
            "num_retries": 3,
            "timeout": 60,
        }

        if self.base_url:
            completion_args["base_url"] = self.base_url
            if self.provider_name == "openai":
                completion_args["custom_llm_provider"] = "openai"
                completion_args["model"] = model
            elif self.provider_name == "cloudflare":
                completion_args["model"] = f"openai/{model}"
                completion_args.pop("custom_llm_provider", None)

        if stream:
            completion_args["stream_options"] = {"include_usage": True}

        if tools and self.provider_name != "cloudflare":
            completion_args["tools"] = tools

        try:
            response = await litellm.acompletion(**completion_args)

            if stream:
                accumulated_tool_calls = {}
                async for chunk in response:
                    # Update usage if available in chunk
                    usage = getattr(chunk, "usage", None) or chunk.get("usage")
                    if usage:
                        yield {"type": "usage", "usage": usage}

                    choices = chunk.get("choices", [])
                    if not choices:
                        continue

                    delta = choices[0].get("delta", {})

                    # Handle text content
                    content = delta.get("content")
                    if content:
                        yield {"type": "text", "content": content}

                    # Handle reasoning content (e.g. DeepSeek, OpenAI O1)
                    reasoning = delta.get("reasoning_content")
                    if reasoning:
                        yield {"type": "reasoning", "content": reasoning}

                    # Handle tool calls
                    tool_calls = delta.get("tool_calls")
                    if tool_calls:
                        for tc in tool_calls:
                            index = tc.get("index", 0)
                            if index not in accumulated_tool_calls:
                                accumulated_tool_calls[index] = {
                                    "id": tc.get("id"),
                                    "type": "function",
                                    "function": {
                                        "name": "",
                                        "arguments": "",
                                    },
                                }

                            f_delta = tc.get("function", {})
                            if f_delta.get("name"):
                                accumulated_tool_calls[index]["function"]["name"] += (
                                    f_delta["name"]
                                )
                            if f_delta.get("arguments"):
                                accumulated_tool_calls[index]["function"][
                                    "arguments"
                                ] += f_delta["arguments"]
                            if tc.get("id"):
                                accumulated_tool_calls[index]["id"] = tc["id"]

                # After stream ends, yield all accumulated tool calls
                for tc in accumulated_tool_calls.values():
                    yield {"type": "tool_call", "tool_call": tc}
            else:
                # Non-streaming mode
                choice = response.choices[0]
                if hasattr(response, "usage") and response.usage:
                    yield {"type": "usage", "usage": response.usage.model_dump()}

                if choice.message.content:
                    yield {"type": "text", "content": choice.message.content}
                if choice.message.tool_calls:
                    for tc in choice.message.tool_calls:
                        yield {"type": "tool_call", "tool_call": tc.model_dump()}

        except Exception as e:
            yield {"type": "error", "content": str(e)}

    async def get_models(self) -> List[Dict[str, Any]]:
        try:
            if self.base_url and (
                self.provider_name == "openai" or self.provider_name == "openrouter"
            ):
                import httpx

                async with httpx.AsyncClient(timeout=10.0) as client:
                    url = self.base_url.rstrip("/") + "/models"
                    headers = {"Authorization": f"Bearer {self.api_key}"}
                    response = await client.get(url, headers=headers)
                    if response.status_code == 200:
                        data = response.json()
                        models_data = data.get("data", [])
                        if models_data:
                            results = []
                            for m in models_data:
                                m_id = m.get("id") if isinstance(m, dict) else str(m)
                                if m_id:
                                    display_name = (
                                        m_id.split("/")[-1] if "/" in m_id else m_id
                                    )
                                    results.append({"id": m_id, "name": display_name})
                            return results

            pattern = get_model_list_pattern(self.provider_name)

            if self.provider_name == "cloudflare":
                import os

                account_id = os.environ.get("CLOUDFLARE_ACCOUNT_ID")
                api_key = os.environ.get("CLOUDFLARE_API_KEY") or self.api_key
                if account_id and api_key:
                    try:
                        import httpx

                        url = f"https://api.cloudflare.com/client/v4/accounts/{account_id}/ai/models/search?task=Text%20Generation"
                        headers = {"Authorization": f"Bearer {api_key}"}
                        async with httpx.AsyncClient(timeout=10.0) as client:
                            resp = await client.get(url, headers=headers)
                            if resp.status_code == 200:
                                data = resp.json()
                                result_data = data.get("result", [])
                                if result_data:
                                    return [
                                        {
                                            "id": m["name"],
                                            "name": m["name"].split("/")[-1],
                                        }
                                        for m in result_data
                                        if "name" in m
                                    ]
                    except Exception:
                        pass

            live_models = await asyncio.to_thread(
                litellm.get_model_list,
                model=pattern,
                api_key=self.api_key,
                base_url=self.base_url,
            )
            if live_models:
                results = []
                for m in live_models:
                    m_id = m if isinstance(m, str) else m.get("id", str(m))
                    display_id = m_id.split("/")[-1] if "/" in m_id else m_id
                    results.append({"id": m_id, "name": display_id})
                return results
        except Exception:
            pass

        if self.models_list:
            return self.models_list

        return []
