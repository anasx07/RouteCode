import json
import asyncio
import litellm
from typing import List, Dict, Any, Optional, AsyncGenerator
from .base import AIProvider

# Disable litellm's verbose logging unless needed
litellm.set_verbose = False
litellm.suppress_debug_info = True

class LiteLLMProvider(AIProvider):
    """
    Standardized AI provider using LiteLLM for cross-provider compatibility.
    """
    def __init__(self, api_key: str, provider_name: str, base_url: Optional[str] = None, models: Optional[List[Dict[str, Any]]] = None):
        super().__init__(api_key)
        self.provider_name = provider_name
        self.base_url = base_url
        self.models_list = models
        # LiteLLM uses provider-specific model names like "anthropic/claude-3-opus-20240229"
        # We'll prepend the provider if it's not already there.

    async def ask(self, messages: List[Dict[str, Any]], model: str, stream: bool = True, tools: Optional[List[Dict[str, Any]]] = None) -> AsyncGenerator[Dict[str, Any], None]:
        # Prepare the model string for LiteLLM
        litellm_model = model
        if self.provider_name:
            # Common LiteLLM provider prefixes
            prefixes = ["openai/", "anthropic/", "gemini/", "deepseek/", "openrouter/", "vertex_ai/", "groq/", "mistral/"]
            has_prefix = any(model.startswith(p) for p in prefixes)
            
            if not has_prefix:
                if self.provider_name == "google":
                    litellm_model = f"gemini/{model}"
                else:
                    litellm_model = f"{self.provider_name}/{model}"

        # LiteLLM needs the API key. We can pass it directly or set it in environment.
        # Passing it in acompletion is safer for multiple providers.
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
                # For custom OpenAI endpoints, use custom_llm_provider to avoid prefix issues
                completion_args["custom_llm_provider"] = "openai"
                completion_args["model"] = model  # Use unprefixed model name

        if stream:
            completion_args["stream_options"] = {"include_usage": True}
        
        if tools:
            completion_args["tools"] = tools
            # LiteLLM handles the translation of tools to the provider's format.

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
                                        "name": tc.get("function", {}).get("name", ""),
                                        "arguments": ""
                                    }
                                }
                            
                            f_delta = tc.get("function", {})
                            if f_delta.get("name"):
                                accumulated_tool_calls[index]["function"]["name"] += f_delta["name"]
                            if f_delta.get("arguments"):
                                accumulated_tool_calls[index]["function"]["arguments"] += f_delta["arguments"]
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
        """
        Return a list of models for the active provider.
        Tries to fetch live models from LiteLLM first, falls back to metadata list.
        """
        # 1. Try to fetch live list from LiteLLM
        try:
            # For custom OpenAI endpoints and OpenRouter, we try a direct fetch
            if self.base_url and (self.provider_name == "openai" or self.provider_name == "openrouter"):
                # Some providers don't support the 'openai/*' pattern in get_model_list
                # but might work if we fetch directly from /models
                import httpx
                async with httpx.AsyncClient(timeout=10.0) as client:
                    # Try standard OpenAI /models endpoint
                    url = self.base_url.rstrip("/") + "/models"
                    headers = {"Authorization": f"Bearer {self.api_key}"}
                    response = await client.get(url, headers=headers)
                    if response.status_code == 200:
                        data = response.json()
                        # Standard OpenAI response is {"data": [...]}
                        models_data = data.get("data", [])
                        if models_data:
                            results = []
                            for m in models_data:
                                m_id = m.get("id") if isinstance(m, dict) else str(m)
                                if m_id:
                                    # Strip prefix for display name (e.g. cohere/command-r -> command-r)
                                    display_name = m_id.split("/")[-1] if "/" in m_id else m_id
                                    results.append({"id": m_id, "name": display_name})
                            return results

            # Standard LiteLLM fetch
            pattern = f"{self.provider_name}/*"
            if self.provider_name == "google": pattern = "gemini/*"
            
            live_models = await asyncio.to_thread(
                litellm.get_model_list,
                model=pattern,
                api_key=self.api_key,
                base_url=self.base_url
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

        # 2. Fallback to models_list (from models_api.json)
        if self.models_list:
            return self.models_list

        # 3. Final hardcoded defaults for common providers
        defaults = {
            "openai": [
                {"id": "gpt-4o", "name": "GPT-4o (Omni)"},
                {"id": "gpt-4o-mini", "name": "GPT-4o Mini"},
                {"id": "gpt-4-turbo", "name": "GPT-4 Turbo"},
                {"id": "o1-preview", "name": "o1-preview (Reasoning)"},
                {"id": "o1-mini", "name": "o1-mini (Reasoning)"},
            ],
            "anthropic": [
                {"id": "claude-3-5-sonnet-20240620", "name": "Claude 3.5 Sonnet"},
                {"id": "claude-3-opus-20240229", "name": "Claude 3 Opus"},
                {"id": "claude-3-haiku-20240307", "name": "Claude 3 Haiku"},
            ],
            "google": [
                {"id": "gemini-1.5-pro", "name": "Gemini 1.5 Pro"},
                {"id": "gemini-1.5-flash", "name": "Gemini 1.5 Flash"},
            ],
            "deepseek": [
                {"id": "deepseek-chat", "name": "DeepSeek Chat"},
                {"id": "deepseek-coder", "name": "DeepSeek Coder"},
            ]
        }
        return defaults.get(self.provider_name, [])
