from typing import Optional
from .events import bus

class TokenizerService:
    """
    Standalone service for tracking token usage and estimating costs.
    Decoupled from the main application state to allow independent orchestration.
    """
    def __init__(self):
        self.tokens_used: int = 0
        self.estimated_cost: float = 0.0

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
        """
        Records token usage. If precise input/output splits are provided, uses them.
        Otherwise treats 'count' as total and estimates 50/50 split.
        """
        from ..utils.costs import cost_estimator

        if input_tokens is not None and output_tokens is not None:
            self.tokens_used += input_tokens + output_tokens
            cost, _, _ = cost_estimator.calculate_cost(input_tokens, output_tokens, model)
        else:
            self.tokens_used += count
            cost, _, _ = cost_estimator.calculate_cost(count // 2, count // 2, model)

        self.estimated_cost += cost
        bus.emit("tokenizer.usage_updated", tokens=self.tokens_used, cost=self.estimated_cost)

    def get_context_usage_percent(self, model: str) -> float:
        from ..utils.costs import cost_estimator
        _, ctx_limit, _ = cost_estimator.calculate_cost(0, 0, model)
        if ctx_limit <= 0:
            return 0.0
        return (self.tokens_used / ctx_limit) * 100

    def load_state(self, tokens: int, cost: float):
        self.tokens_used = tokens
        self.estimated_cost = cost