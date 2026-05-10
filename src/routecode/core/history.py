from collections import deque
from typing import List, Dict, Any, Optional, Deque
from .events import bus


class ConversationHistory:
    """
    Unified manager for conversation messages.

    Uses a deque with a configurable max size to bound memory growth.
    Safe mutation methods emit events for downstream listeners.
    """

    def __init__(
        self,
        messages: Optional[List[Dict[str, Any]]] = None,
        maxlen: int = 2000,
    ):
        self._messages: Deque[Dict[str, Any]] = deque(
            messages if messages is not None else [],
            maxlen=maxlen,
        )

    def append(self, message: Dict[str, Any]):
        self._messages.append(message)
        bus.emit("history.appended", message=message)

    def extend(self, messages: List[Dict[str, Any]]):
        for m in messages:
            self._messages.append(m)
            bus.emit("history.appended", message=m)

    def clear(self):
        self._messages.clear()
        bus.emit("history.cleared")

    def rewind(self, count: int):
        """Removes the last N messages."""
        if count <= 0:
            return
        for _ in range(min(count, len(self._messages))):
            self._messages.pop()
        bus.emit("history.rewound", count=count)

    def set_messages(self, messages: Any):
        """Completely replaces the history."""
        if isinstance(messages, ConversationHistory):
            source = messages.get_messages()
        else:
            source = list(messages)
        self._messages.clear()
        for m in source:
            self._messages.append(m)
        bus.emit("history.reset")

    def get_messages(self) -> List[Dict[str, Any]]:
        """Returns the raw list for iteration or API calls."""
        return list(self._messages)

    def to_list(self) -> List[Dict[str, Any]]:
        """Returns a shallow copy as a list."""
        return list(self._messages)

    def snapshot(self) -> List[Dict[str, Any]]:
        """Alias for to_list()."""
        return self.to_list()

    def __len__(self):
        return len(self._messages)

    def __getitem__(self, index):
        if isinstance(index, slice):
            return list(self._messages)[index]
        return self._messages[index]

    def __iter__(self):
        return iter(self._messages)
