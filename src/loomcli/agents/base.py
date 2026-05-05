from abc import ABC, abstractmethod
from typing import List, Dict, Optional, Any, Generator
from .transport import SSETransport

class AIProvider(ABC):
    def __init__(self, api_key: str):
        self.api_key = api_key
        self.transport = SSETransport()

    @abstractmethod
    def ask(self, messages: List[Dict[str, str]], model: str, stream: bool = True, tools: Optional[List[Dict[str, Any]]] = None) -> Generator[Dict[str, Any], None, None]:
        """
        Send a prompt to the AI provider and return a generator for the response chunks.
        """
        pass

    def close(self):
        self.transport.close()
