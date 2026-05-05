import time
from dataclasses import dataclass, field
from typing import Dict, Optional, List

# Minimal fallback for core models when the database is unavailable or missing a entry.
# Primary pricing should always come from models_db.py (models_api.json).
FALLBACK_PRICING = {
    "gpt-4o": (2.50, 10.00, 128000),
    "claude-3.5-sonnet": (3.00, 15.00, 200000),
    "gemini-1.5-pro": (1.25, 5.00, 1048576),
    "deepseek-chat": (0.27, 1.10, 65536),
}

DEFAULT_INPUT_PRICE = 2.00
DEFAULT_OUTPUT_PRICE = 10.00
DEFAULT_CONTEXT_LIMIT = 32000


def get_model_pricing(model: str) -> tuple:
    """Retrieves pricing info for a model, prioritizing the models_db."""
    try:
        from .models_db import get_model_pricing as db_pricing
        prices = db_pricing(model)
        # If the DB returned non-default values, we assume it found the model
        if prices[0] != DEFAULT_INPUT_PRICE or prices[2] != DEFAULT_CONTEXT_LIMIT:
            return prices
    except Exception:
        pass

    # Fallback to a very small list of well-known models
    for key, prices in FALLBACK_PRICING.items():
        if key in model.lower():
            return prices
            
    return (DEFAULT_INPUT_PRICE, DEFAULT_OUTPUT_PRICE, DEFAULT_CONTEXT_LIMIT)


def count_tokens(text: str, model: Optional[str] = None) -> int:
    try:
        import tiktoken
        try:
            encoding = tiktoken.encoding_for_model(model or "gpt-4")
        except KeyError:
            encoding = tiktoken.get_encoding("cl100k_base")
        return len(encoding.encode(text))
    except Exception:
        # Fallback: simple word and punctuation tokenizer. Provides a much safer 
        # upper-bound estimate for code and JSON compared to simple split().
        import re
        tokens = re.findall(r'\w+|[^\w\s]', text)
        return len(tokens)


@dataclass
class SessionState:
    tokens_used: int = 0
    estimated_cost: float = 0.0
    commands_run: int = 0
    tools_called: int = 0
    start_time: float = 0.0
    session_messages: List[Dict] = field(default_factory=list)
    context_warned: bool = False

    def add_tokens(self, count: int, model: Optional[str] = None):
        self.tokens_used += count
        input_price, output_price, ctx_limit = get_model_pricing(model or "")
        avg_price = (input_price + output_price) / 2
        self.estimated_cost += (count / 1_000_000) * avg_price

        if not self.context_warned and ctx_limit > 0:
            pct = self.tokens_used / ctx_limit * 100
            if pct > 70:
                self.context_warned = True
                return pct
        return None

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

