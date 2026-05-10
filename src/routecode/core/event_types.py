"""
Typed event dataclasses for the RouteCode EventBus.

Each dataclass represents a distinct event with typed fields.
Use with bus.emit_typed() / bus.on_typed() for static analysis,
IDE autocompletion, and protection against typos.
"""

from dataclasses import dataclass
from typing import Any, Dict, Optional


# ── Config events ───────────────────────────────────────────────────────

@dataclass
class ProviderChanged:
    provider: str


# ── Context management events ───────────────────────────────────────────

@dataclass
class ContextThresholdWarning:
    usage: float
    type: str  # "micro" | "full"


@dataclass
class ContextCompacted:
    type: str  # "micro"
    saved: int


# ── History events ──────────────────────────────────────────────────────

@dataclass
class HistoryAppended:
    message: Dict[str, Any]


@dataclass
class HistoryCleared:
    pass


@dataclass
class HistoryRewound:
    count: int


@dataclass
class HistoryReset:
    pass


# ── Session events ──────────────────────────────────────────────────────

@dataclass
class SessionReset:
    pass


# ── Task events ─────────────────────────────────────────────────────────

@dataclass
class TaskCreated:
    task_id: str
    description: str


@dataclass
class TaskCompleted:
    task_id: str
    description: str
    result: Optional[Dict[str, Any]] = None


@dataclass
class TaskFailed:
    task_id: str
    description: str
    error: str


# ── Tokenizer events ────────────────────────────────────────────────────

@dataclass
class TokenUsageUpdated:
    tokens: int
    cost: float


# ── UI events ───────────────────────────────────────────────────────────

@dataclass
class ThemeChanged:
    name: str
