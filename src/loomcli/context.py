from dataclasses import dataclass
from rich.console import Console
from .state import SessionState
from .config import Config
from .task_manager import TaskManager
from .memory import MemoryManager

@dataclass
class LoomContext:
    """
    Unified context object for passing dependencies to commands and tools.
    Eliminates the need for reflection-based parameter detection.
    """
    state: SessionState
    config: Config
    console: Console
    task_manager: TaskManager
    memory: MemoryManager
