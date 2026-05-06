import json
import asyncio
import litellm
from typing import List, Dict, Any, Optional, AsyncGenerator
from .base import AIProvider

# Disable litellm's verbose logging unless needed
litellm.set_verbose = False

class LiteLLMProvider(AIProvider):
    """
    Standardized AI provider using LiteLLM for cross-provider compatibility.
    """
    def __init__(self, api_key: str, provider_name: str):
        super().__init__(api_key)
        self.provider_name = provider_name
        # LiteLLM uses provider-specific model names like "anthropic/claude-3-opus-20240229"
        # We'll prepend the provider if it's not already there.

    async def ask(self, messages: List[Dict[str, Any]], model: str, stream: bool = True, tools: Optional[List[Dict[str, Any]]] = None) -> AsyncGenerator[Dict[str, Any], None]:
        # Prepare the model string for LiteLLM if it doesn't already have a prefix
        litellm_model = model
        if "/" not in model and self.provider_name:
            if self.provider_name == "google":
                 litellm_model = f"gemini/{model}"
            elif self.provider_name == "openai":
                 litellm_model = f"openai/{model}"
            elif self.provider_name == "anthropic":
                 litellm_model = f"anthropic/{model}"
            elif self.provider_name == "deepseek":
                 litellm_model = f"deepseek/{model}"
            elif self.provider_name == "openrouter":
                 litellm_model = f"openrouter/{model}"

        # LiteLLM needs the API key. We can pass it directly or set it in environment.
        # Passing it in acompletion is safer for multiple providers.
        completion_args = {
            "model": litellm_model,
            "messages": messages,
            "stream": stream,
            "api_key": self.api_key,
        }

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
        Return a list of common models for the active provider.
        """
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
                {"id": "gemini-3.1-pro-preview", "name": "Gemini 3.1 Pro Preview"},
                {"id": "gemini-3-flash-preview", "name": "Gemini 3 Flash Preview"},
                {"id": "gemini-3.1-flash-lite-preview", "name": "Gemini 3.1 Flash lite Preview"}
            ],
            "deepseek": [
                {"id": "deepseek-chat", "name": "DeepSeek Chat"},
                {"id": "deepseek-coder", "name": "DeepSeek Coder"},
            ]
        }
        return defaults.get(self.provider_name, [])
