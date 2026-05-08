import time
from dataclasses import dataclass, field
from typing import Optional

from .history import ConversationHistory
from ..utils.storage import AtomicJsonStore
from ..config import CONFIG_DIR


def count_tokens(text: str, model: Optional[str] = None) -> int:
    """Delegates to the centralized cost_estimator."""
    from ..utils.costs import cost_estimator

    return cost_estimator.count_tokens(text, model or "gpt-4")


@dataclass
class SessionState:
    tokens_used: int = 0
    estimated_cost: float = 0.0
    commands_run: int = 0
    tools_called: int = 0
    start_time: float = 0.0
    session_messages: ConversationHistory = field(default_factory=ConversationHistory)
    session_allowlist: list = field(default_factory=list)
    context_warned: bool = False

    provider: Optional[str] = None
    model: Optional[str] = None

    def bind_tokenizer(self, tokenizer):
        self._tokenizer = tokenizer
        from .events import bus

        bus.on("tokenizer.usage_updated", self._on_usage_updated)

    def _on_usage_updated(self, tokens: int, cost: float, **kwargs):
        self.tokens_used = tokens
        self.estimated_cost = cost

    def get_context_usage(self, model: str) -> float:
        """Returns the current context usage percentage."""
        if hasattr(self, "_tokenizer") and self._tokenizer:
            return self._tokenizer.get_context_usage_percent(model)

        from ..utils.costs import cost_estimator

        _, ctx_limit, _ = cost_estimator.calculate_cost(0, 0, model)
        if ctx_limit <= 0:
            return 0.0
        return (self.tokens_used / ctx_limit) * 100

    def reset_context_warning(self):
        """Resets the context warning flag, typically after compaction."""
        self.context_warned = False

    def to_dict(self) -> dict:
        return {
            "tokens_used": self.tokens_used,
            "estimated_cost": self.estimated_cost,
            "commands_run": self.commands_run,
            "tools_called": self.tools_called,
            "provider": self.provider,
            "model": self.model,
            "messages": self.session_messages.to_list(),
            "session_allowlist": self.session_allowlist,
        }

    @classmethod
    def from_dict(cls, data: dict) -> "SessionState":
        return cls(
            tokens_used=data.get("tokens_used", 0),
            estimated_cost=data.get("estimated_cost", 0.0),
            commands_run=data.get("commands_run", 0),
            tools_called=data.get("tools_called", 0),
            provider=data.get("provider"),
            model=data.get("model"),
            session_messages=ConversationHistory(data.get("messages", [])),
            session_allowlist=data.get("session_allowlist", []),
        )

    def merge(self, other: "SessionState"):
        """Aggregates statistics from another session state (e.g. from a sub-agent)."""
        self.tokens_used += other.tokens_used
        self.estimated_cost += other.estimated_cost
        self.commands_run += other.commands_run
        self.tools_called += other.tools_called


SESSIONS_DIR = CONFIG_DIR / "sessions"


def save_session(state: SessionState, name: str):
    """Save a session state atomically."""
    path = SESSIONS_DIR / f"{name}.json"
    store = AtomicJsonStore(path)
    data = state.to_dict()
    data["saved_at"] = time.strftime("%Y-%m-%d %H:%M:%S")
    store.save(data)


async def save_session_async(state: SessionState, name: str):
    """Asynchronously save a session state atomically."""
    path = SESSIONS_DIR / f"{name}.json"
    store = AtomicJsonStore(path)
    data = state.to_dict()
    data["saved_at"] = time.strftime("%Y-%m-%d %H:%M:%S")
    await store.save_async(data)


def load_session(name: str) -> Optional[SessionState]:
    """Load a session state from a file."""
    path = SESSIONS_DIR / f"{name}.json"
    store = AtomicJsonStore(path)
    data = store.load()
    if not data:
        return None
    return SessionState.from_dict(data)


async def load_session_async(name: str) -> Optional[SessionState]:
    """Asynchronously load a session state from a file."""
    path = SESSIONS_DIR / f"{name}.json"
    store = AtomicJsonStore(path)
    data = await store.load_async()
    if not data:
        return None
    return SessionState.from_dict(data)
