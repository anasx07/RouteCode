import asyncio
from pathlib import Path
from typing import Optional, TYPE_CHECKING

if TYPE_CHECKING:
    from .events import EventBus
    from .state import SessionState
    from .path_guard import PathGuard
    from .memory import MemoryManager
    from .tokenizer import TokenizerService
    from .context import RouteCodeContext
    from ..config import Config
    from ..domain.task_manager import TaskManager


class AppContainer:
    """
    Service container with explicit initialization phases.

    Phase 1: build()     — Create session-scoped services, wire module-level globals
    Phase 2: initialize() — Load persisted state (async)
    Phase 3: validate()  — Assert all services ready
    Phase 4: start()     — Hook into event loop
    Phase 5: shutdown()  — Clean teardown

    Module-level globals (config, registry, cost_estimator) are imported
    and referenced directly rather than duplicated, so code that imports
    them continues to work unchanged.
    """

    def __init__(self, config_dir: Path):
        self._config_dir = config_dir
        self._built = False
        self._initialized = False

        # Tier 1: Module-level globals (reference, not duplicate)
        self.bus: Optional["EventBus"] = None
        self.config: Optional["Config"] = None

        # Tier 2: Session-scoped services (fresh per session)
        self.state: Optional["SessionState"] = None
        self.path_guard: Optional["PathGuard"] = None
        self.memory: Optional["MemoryManager"] = None

        # Tier 3: Lazy runtime services
        self._tokenizer: Optional["TokenizerService"] = None
        self._task_manager: Optional["TaskManager"] = None
        self._orchestrator: Optional["AgentOrchestrator"] = None
        self._ctx: Optional["RouteCodeContext"] = None

    def build(self) -> "AppContainer":
        from .events import EventBus
        from .state import SessionState
        from .path_guard import PathGuard
        from .memory import MemoryManager

        # Tier 1: EventBus (new instance replaces module-level global)
        self.bus = EventBus()
        self._connect_bus()

        # Config — reference the existing module-level singleton
        from ..config import config as global_config

        self.config = global_config
        self.config.store.cleanup_stale_temps()

        # Tier 2: Session-scoped services
        self.state = SessionState()
        self.path_guard = PathGuard()
        self.memory = MemoryManager(self._config_dir / "memory")

        self._built = True
        return self

    def _connect_bus(self):
        """Replace the module-level EventBus singleton with the container's instance."""
        from . import events as events_mod

        events_mod.bus = self.bus

    async def initialize(self) -> "AppContainer":
        if not self._built:
            raise RuntimeError("build() must be called before initialize()")
        await self.memory._load_async()
        self._initialized = True
        return self

    def validate(self) -> "AppContainer":
        if not self._initialized:
            raise RuntimeError("initialize() must be called before validate()")
        assert self.bus is not None, "EventBus not created"
        assert self.config is not None, "Config not created"
        assert self.state is not None, "SessionState not created"
        assert self.memory is not None, "MemoryManager not created"
        return self

    def set_event_loop(self, loop: asyncio.AbstractEventLoop):
        self.ctx.loop = loop

    def shutdown(self):
        self.bus.clear()
        self._orchestrator = None
        self._tokenizer = None
        self._ctx = None
        self._initialized = False
        self._built = False

    @property
    def tokenizer(self) -> "TokenizerService":
        if self._tokenizer is None:
            from .tokenizer import TokenizerService

            self._tokenizer = TokenizerService(bus=self.bus)
            self.state.bind_tokenizer(self._tokenizer, bus=self.bus)
        return self._tokenizer

    @property
    def task_manager(self) -> "TaskManager":
        if self._task_manager is None:
            from ..domain.task_manager import TaskManager

            self._task_manager = TaskManager()
        return self._task_manager

    @property
    def orchestrator(self) -> "AgentOrchestrator":
        if self._orchestrator is None:
            from .orchestrator import AgentOrchestrator

            self._orchestrator = AgentOrchestrator(self.ctx)
        return self._orchestrator

    @property
    def ctx(self) -> "RouteCodeContext":
        if self._ctx is None:
            from .context import RouteCodeContext
            from ..ui.console import console

            self._ctx = RouteCodeContext(
                state=self.state,
                config=self.config,
                console=console,
                task_manager=self.task_manager,
                memory=self.memory,
                path_guard=self.path_guard,
                bus=self.bus,
            )
        return self._ctx
