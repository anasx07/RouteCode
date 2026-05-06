from typing import List, Dict, Any, Optional
from .events import bus

class ConversationHistory:
    """
    Unified manager for conversation messages.
    Wraps a list of messages and provides safe mutation methods.
    """
    def __init__(self, messages: Optional[List[Dict[str, Any]]] = None):
        self._messages: List[Dict[str, Any]] = messages if messages is not None else []

    def append(self, message: Dict[str, Any]):
        self._messages.append(message)
        bus.emit("history.appended", message=message)

    def extend(self, messages: List[Dict[str, Any]]):
        self._messages.extend(messages)
        for m in messages:
            bus.emit("history.appended", message=m)

    def clear(self):
        self._messages.clear()
        bus.emit("history.cleared")

    def rewind(self, count: int):
        """Removes the last N turns/messages."""
        if count <= 0:
            return
        self._messages = self._messages[:-count]
        bus.emit("history.rewound", count=count)

    def set_messages(self, messages: Any):
        """Completely replaces the history."""
        if isinstance(messages, ConversationHistory):
            self._messages = messages.get_messages()
        else:
            self._messages = list(messages)
        bus.emit("history.reset")

    def get_messages(self) -> List[Dict[str, Any]]:
        """Returns the raw list for iteration or API calls."""
        return self._messages

    def to_list(self) -> List[Dict[str, Any]]:
        """Returns a copy of the underlying list."""
        return self._messages[:]

    def snapshot(self) -> List[Dict[str, Any]]:
        """Alias for to_list()."""
        return self.to_list()

    def __len__(self):
        return len(self._messages)

    def __getitem__(self, index):
        return self._messages[index]

    def __iter__(self):
        return iter(self._messages)
