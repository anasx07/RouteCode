from abc import ABC, abstractmethod
from typing import List, Dict, Any, Optional, Union
from pydantic import BaseModel

class LoomMessage(BaseModel):
    """
    Canonical internal message format used by LoomCLI.
    Closely follows the OpenAI format but abstracted for multi-provider support.
    """
    role: str           # "system", "user", "assistant", "tool"
    content: Optional[Union[str, List[Dict[str, Any]]]] = None
    tool_calls: Optional[List[Dict[str, Any]]] = None
    tool_call_id: Optional[str] = None
    name: Optional[str] = None

class MessageAdapter(ABC):
    """
    Base class for adapting LoomMessages to provider-specific formats 
    and parsing provider responses back into LoomMessages.
    """
    @abstractmethod
    def to_provider(self, messages: List[LoomMessage]) -> Any:
        """Convert a list of LoomMessages to the provider's native format."""
        pass

    @abstractmethod
    def to_provider_tools(self, tools: List[Dict[str, Any]]) -> Any:
        """Convert standard OpenAI-style tool schemas to the provider's format."""
        pass
