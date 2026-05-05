from typing import Callable, Dict, List, Any
import logging

logger = logging.getLogger(__name__)

class EventBus:
    """
    A lightweight, thread-safe event bus for decoupling LoomCLI subsystems.
    Allows modules to emit events without knowing about their listeners.
    """
    def __init__(self):
        self._handlers: Dict[str, List[Callable]] = {}

    def on(self, event: str, handler: Callable):
        """Registers a handler for a specific event."""
        if event not in self._handlers:
            self._handlers[event] = []
        self._handlers[event].append(handler)

    def emit(self, event: str, **data):
        """Emits an event and notifies all registered handlers."""
        if event not in self._handlers:
            return
            
        for handler in self._handlers[event]:
            try:
                handler(**data)
            except Exception as e:
                logger.error(f"Error in event handler for {event}: {e}")

# Global bus instance
bus = EventBus()
