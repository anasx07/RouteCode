from typing import Optional, TYPE_CHECKING

if TYPE_CHECKING:
    from .events import EventBus


class TokenizerService:
    """
    Single source of truth for token counting, cost estimation, and context usage.

    All token queries pass through this service. State binding via
    SessionState.bind_tokenizer() subscribes to usage events emitted here.
    """

    def __init__(self, bus: Optional["EventBus"] = None):
        self.tokens_used: int = 0
        self.estimated_cost: float = 0.0
        self._bus = bus

    def _get_bus(self):
        if self._bus is not None:
            return self._bus
        from .events import bus

        return bus

    def count_tokens(self, text: str, model: str) -> int:
        from ..utils.costs import cost_estimator

        return cost_estimator.count_tokens(text, model)

    def add_usage(
        self,
        count: int,
        model: str,
        input_tokens: Optional[int] = None,
        output_tokens: Optional[int] = None,
    ):
        from ..utils.costs import cost_estimator

        if input_tokens is not None and output_tokens is not None:
            self.tokens_used += input_tokens + output_tokens
            cost, _, _ = cost_estimator.calculate_cost(
                input_tokens, output_tokens, model
            )
        else:
            self.tokens_used += count
            cost, _, _ = cost_estimator.calculate_cost(count // 2, count // 2, model)

        self.estimated_cost += cost
        self._get_bus().emit(
            "tokenizer.usage_updated", tokens=self.tokens_used, cost=self.estimated_cost
        )

    def get_context_usage_percent(self, model: str) -> float:
        from ..utils.costs import cost_estimator

        _, ctx_limit, _ = cost_estimator.calculate_cost(0, 0, model)
        if ctx_limit <= 0:
            return 0.0
        return (self.tokens_used / ctx_limit) * 100

    def recalculate(self, content: str, model: str):
        """Recalculate token count and cost from content string (used after compaction)."""
        new_count = self.count_tokens(content, model)
        self.load_state(new_count, self._reestimate_cost(new_count, model))
        self._get_bus().emit(
            "tokenizer.usage_updated", tokens=self.tokens_used, cost=self.estimated_cost
        )

    def _reestimate_cost(self, token_count: int, model: str) -> float:
        from ..utils.costs import cost_estimator

        cost, _, _ = cost_estimator.calculate_cost(
            token_count // 2, token_count // 2, model
        )
        return cost

    def load_state(self, tokens: int, cost: float):
        self.tokens_used = tokens
        self.estimated_cost = cost
