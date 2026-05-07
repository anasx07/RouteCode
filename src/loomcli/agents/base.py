from abc import ABC, abstractmethod
from typing import List, Dict, Optional, Any, AsyncGenerator


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
    ) -> AsyncGenerator[Dict[str, Any], None]:
        """
        Send a prompt to the AI provider and return an async generator for the response chunks.
        """
        yield {}  # Placeholder for abstract method

    async def close(self):
        """Cleanup resources."""
        pass
