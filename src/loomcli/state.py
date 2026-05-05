import time
from dataclasses import dataclass, field
from typing import Dict, Optional, List

MODEL_PRICING = {
    "gpt-4o": (2.50, 10.00, 128000),
    "gpt-4o-2024-08-06": (2.50, 10.00, 128000),
    "gpt-4o-mini": (0.15, 0.60, 128000),
    "gpt-4-turbo": (10.00, 30.00, 128000),
    "gpt-4": (30.00, 60.00, 8192),
    "gpt-3.5-turbo": (0.50, 1.50, 16384),
    "claude-3.5-sonnet": (3.00, 15.00, 200000),
    "claude-3-sonnet": (3.00, 15.00, 200000),
    "claude-3-haiku": (0.25, 1.25, 200000),
    "claude-3-opus": (15.00, 75.00, 200000),
    "claude-3.5-haiku": (0.80, 4.00, 200000),
    "deepseek-chat": (0.27, 1.10, 65536),
    "deepseek-reasoner": (0.55, 2.19, 65536),
    "gemini-1.5-pro": (1.25, 5.00, 1048576),
    "gemini-1.5-flash": (0.075, 0.30, 1048576),
    "gemini-2.0-flash": (0.10, 0.40, 1048576),
    "mistral-large": (4.00, 12.00, 128000),
    "mistral-medium": (2.70, 8.10, 32000),
}

DEFAULT_INPUT_PRICE = 2.00
DEFAULT_OUTPUT_PRICE = 10.00
DEFAULT_CONTEXT_LIMIT = 32000


def get_model_pricing(model: str) -> tuple:
    try:
        from .models_db import get_model_pricing as db_pricing
        prices = db_pricing(model)
        if prices[0] != DEFAULT_INPUT_PRICE:
            return prices
    except Exception:
        pass

    for key, prices in MODEL_PRICING.items():
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
        return int(len(text.split()) * 1.3 + len(text) / 4)


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

state = SessionState()
