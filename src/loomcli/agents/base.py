from abc import ABC, abstractmethod
from typing import Generator, List, Dict, Any, Optional

class AIProvider(ABC):
    @abstractmethod
    def ask(self, messages: List[Dict[str, str]], model: str, stream: bool = True, tools: Optional[List[Dict[str, Any]]] = None) -> Generator[Dict[str, Any], None, None]:
        """
        Send a prompt to the AI provider and return a generator for the response chunks.
        """
        pass
