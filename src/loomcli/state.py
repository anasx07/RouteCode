import time
from dataclasses import dataclass, field
from typing import Dict, Optional, List
from .costs import cost_estimator

def count_tokens(text: str, model: Optional[str] = None) -> int:
    """Delegates to the centralized cost_estimator."""
    return cost_estimator.count_tokens(text, model or "gpt-4")


@dataclass
class SessionState:
    tokens_used: int = 0
    estimated_cost: float = 0.0
    commands_run: int = 0
    tools_called: int = 0
    start_time: float = 0.0
    session_messages: List[Dict] = field(default_factory=list)
    context_warned: bool = False

    def add_tokens(self, count: int, model: Optional[str] = None, input_tokens: Optional[int] = None, output_tokens: Optional[int] = None):
        """
        Updates session statistics with new token usage.
        If input_tokens/output_tokens are provided, uses them for precise costing.
        Otherwise, treats 'count' as total and estimates a 50/50 split.
        """
        if input_tokens is not None and output_tokens is not None:
            self.tokens_used += (input_tokens + output_tokens)
            cost, ctx_limit = cost_estimator.calculate_cost(input_tokens, output_tokens, model or "")
        else:
            self.tokens_used += count
            # Estimate 50/50 split if only total is provided
            cost, ctx_limit = cost_estimator.calculate_cost(count // 2, count // 2, model or "")
        
        self.estimated_cost += cost

        if not self.context_warned and ctx_limit > 0:
            pct = self.tokens_used / ctx_limit * 100
            if pct > 70:
                self.context_warned = True
                return pct
        return None

    def get_context_usage(self, model: str) -> float:
        """Returns the current context usage percentage."""
        _, _, ctx_limit = cost_estimator.calculate_cost(0, 0, model)
        if ctx_limit <= 0: return 0.0
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
            "messages": self.session_messages,
        }

    @classmethod
    def from_dict(cls, data: dict) -> 'SessionState':
        return cls(
            tokens_used=data.get("tokens_used", 0),
            estimated_cost=data.get("estimated_cost", 0.0),
            commands_run=data.get("commands_run", 0),
            tools_called=data.get("tools_called", 0),
            session_messages=data.get("messages", []),
        )


from .storage import AtomicJsonStore
from .config import CONFIG_DIR

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

