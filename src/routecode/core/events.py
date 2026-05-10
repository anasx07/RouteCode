from typing import Callable, Dict, List, Any, Optional, Type
import logging
import asyncio

logger = logging.getLogger(__name__)


class EventBus:
    """
    A lightweight, thread-safe event bus for decoupling routecode subsystems.

    Supports both string-based events (emit/on) and typed dataclass events
    (emit_typed/on_typed) for improved static analysis and type safety.
    """

    def __init__(self):
        self._handlers: Dict[str, List[Callable]] = {}
        self._typed_to_name: Dict[type, str] = {}

    def on(self, event: str, handler: Callable):
        """Registers a handler for a specific event name."""
        if event not in self._handlers:
            self._handlers[event] = []
        if handler not in self._handlers[event]:
            self._handlers[event].append(handler)

    def on_typed(self, event_class: type, handler: Callable):
        """Registers a handler for a typed event dataclass."""
        name = self._resolve_typed_name(event_class)
        self._typed_to_name[event_class] = name
        self.on(name, handler)

    def off(self, event: str, handler: Callable):
        """Deregisters a handler for a specific event."""
        if event in self._handlers and handler in self._handlers[event]:
            self._handlers[event].remove(handler)

    def clear(self, event: Optional[str] = None):
        """Clears all handlers for a specific event, or all events if none specified."""
        if event:
            if event in self._handlers:
                self._handlers[event] = []
        else:
            self._handlers = {}
            self._typed_to_name.clear()

    def emit(self, event: str, **data):
        """
        Synchronously emits a string-named event.
        All handlers must be synchronous.
        """
        if event not in self._handlers:
            return

        for handler in self._handlers[event]:
            try:
                handler(**data)
            except Exception as e:
                logger.error(f"Error in sync event handler for {event}: {e}")

    def emit_typed(self, event):
        """
        Synchronously emits a typed dataclass event.
        Converts the event fields to kwargs and dispatches to all registered
        handlers (both typed and string-based).
        """
        name = self._resolve_typed_name(type(event))
        self.emit(name, **vars(event))

    async def emit_async(self, event: str, **data):
        """
        Asynchronously emits an event. Properly awaits all async handlers.
        Sync handlers are called directly.
        """
        if event not in self._handlers:
            return

        for handler in self._handlers[event]:
            try:
                if asyncio.iscoroutinefunction(handler):
                    await handler(**data)
                else:
                    handler(**data)
            except Exception as e:
                logger.error(f"Error in async event handler for {event}: {e}")

    @staticmethod
    def _resolve_typed_name(event_class: type) -> str:
        """Derives a stable event name from a typed event class."""
        module = getattr(event_class, "__module__", "")
        qualname = getattr(event_class, "__qualname__", event_class.__name__)
        if module and module != "builtins":
            # core.event_types.ProviderChanged -> core.event_types.ProviderChanged
            return f"{module}.{qualname}"
        return qualname


# Global bus instance
bus = EventBus()
