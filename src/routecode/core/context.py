import asyncio
from typing import Optional, TYPE_CHECKING
from dataclasses import dataclass
from rich.console import Console
from .state import SessionState
from ..config import Config
from .memory import MemoryManager
from .path_guard import PathGuard

if TYPE_CHECKING:
    from ..domain.task_manager import TaskManager
    from .events import EventBus


@dataclass
class RouteCodeContext:
    """
    Unified context object for passing dependencies to commands and tools.
    Eliminates the need for reflection-based parameter detection.
    """

    state: SessionState
    config: Config
    console: Console
    task_manager: "TaskManager"
    memory: MemoryManager
    path_guard: PathGuard
    bus: "EventBus" = None
    loop: Optional[asyncio.AbstractEventLoop] = None
