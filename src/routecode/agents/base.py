from abc import ABC, abstractmethod
from typing import List, Dict, Optional, Any, AsyncGenerator
from .types import StreamChunk


class AIProvider(ABC):
    def __init__(self, api_key: str):
        self.api_key = api_key

    @abstractmethod
    async def ask(
        self,
        messages: List[Dict[str, str]],
        model: str,
        stream: bool = True,
        tools: Optional[List[Dict[str, Any]]] = None,
    ) -> AsyncGenerator[StreamChunk, None]:
        """
        Send a prompt to the AI provider and return an async generator
        yielding typed StreamChunk events.

        Chunk types:
          - {"type": "text",      "content": str}      — response text
          - {"type": "thought",    "content": str}      — reasoning tokens
          - {"type": "tool_call",  "tool_call": dict}   — function call
          - {"type": "usage",      "usage": dict}       — token usage
          - {"type": "error",      "content": str}      — fatal error
        """
        yield {}  # Placeholder for abstract method

    async def close(self):
        """Cleanup resources."""
        pass
