from typing import Callable, Dict, List, Any, Optional
import logging
import asyncio
import inspect
from weakref import WeakSet

logger = logging.getLogger(__name__)


class EventBus:
    """
    A lightweight, thread-safe event bus for decoupling routecode subsystems.
    Allows modules to emit events without knowing about their listeners.
    """

    def __init__(self):
        self._handlers: Dict[str, List[Callable]] = {}
        self._active_tasks = WeakSet()

    def on(self, event: str, handler: Callable):
        """Registers a handler for a specific event."""
        if event not in self._handlers:
            self._handlers[event] = []
        if handler not in self._handlers[event]:
            self._handlers[event].append(handler)

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

    def emit(self, event: str, **data):
        """
        Synchronously emits an event. Async handlers are fired as background tasks.
        """
        if event not in self._handlers:
            return

        for handler in self._handlers[event]:
            try:
                if inspect.iscoroutinefunction(handler):
                    self._fire_and_forget(handler, data, event)
                else:
                    handler(**data)
            except Exception as e:
                logger.error(f"Error in sync event handler for {event}: {e}")

    async def emit_async(self, event: str, **data):
        """
        Asynchronously emits an event. Properly awaits all async handlers.
        """
        if event not in self._handlers:
            return

        tasks = []
        for handler in self._handlers[event]:
            try:
                if inspect.iscoroutinefunction(handler):
                    tasks.append(handler(**data))
                else:
                    handler(**data)
            except Exception as e:
                logger.error(f"Error in async event handler for {event}: {e}")

        if tasks:
            results = await asyncio.gather(*tasks, return_exceptions=True)
            for res in results:
                if isinstance(res, Exception):
                    logger.error(f"Async error in handler for {event}: {res}")

    def _fire_and_forget(
        self, handler: Callable, data: Dict[str, Any], event_name: str
    ):
        try:
            loop = asyncio.get_running_loop()
            task = loop.create_task(handler(**data))
            self._active_tasks.add(task)
            task.add_done_callback(lambda t: self._active_tasks.discard(t))

            def _log_error(t):
                try:
                    t.result()
                except Exception as e:
                    logger.error(f"Background task error for {event_name}: {e}")

            task.add_done_callback(_log_error)
        except RuntimeError:
            pass


# Global bus instance
bus = EventBus()
