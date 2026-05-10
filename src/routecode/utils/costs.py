import litellm
import re
from typing import Tuple, Optional
from .logger import get_logger
from ..config.models_db import get_model_pricing

logger = get_logger(__name__)


class CostEstimator:
    """
    Decoupled service for calculating token counts and financial costs.
    Leverages litellm for robust tokenization and pricing data.
    """

    def __init__(
        self, default_input_price: float = 2.0, default_output_price: float = 10.0
    ):
        self.default_input_price = default_input_price
        self.default_output_price = default_output_price
        self._model_info_cache: dict = {}
        self._model_info_failed: set = set()

    def count_tokens(self, text: str, model: str) -> int:
        try:
            return litellm.token_counter(model=model, text=text)
        except Exception as e:
            logger.debug(
                "LiteLLM token_counter failed for %s: %s. Using regex fallback.",
                model,
                e,
            )
            tokens = re.findall(r"\w+|[^\w\s]", text)
            return len(tokens)

    def _get_model_info(self, model: str) -> Optional[dict]:
        """Cached wrapper around litellm.get_model_info to avoid blocking
        the event loop with repeated synchronous network calls."""
        if model in self._model_info_cache:
            return self._model_info_cache[model]
        if model in self._model_info_failed:
            return None

        try:
            info = litellm.get_model_info(model)
            self._model_info_cache[model] = info
            return info
        except Exception as e:
            logger.debug(
                "Failed to get model info from LiteLLM for %s: %s", model, e
            )
            self._model_info_failed.add(model)
            return None

    def calculate_cost(
        self, input_tokens: int, output_tokens: int, model: str
    ) -> Tuple[float, int, float]:
        model_info = self._get_model_info(model)
        if model_info:
            input_price = model_info.get("input_cost_per_token")
            output_price = model_info.get("output_cost_per_token")
            context_limit = model_info.get("max_tokens", 32000)

            if input_price is not None and output_price is not None:
                cost = (input_tokens * input_price) + (output_tokens * output_price)
                return cost, context_limit, input_price * 1_000_000

        # Fallback to internal models_db
        input_price_m, output_price_m, context_limit = get_model_pricing(model)
        cost = (input_tokens * input_price_m / 1_000_000) + (
            output_tokens * output_price_m / 1_000_000
        )
        return cost, context_limit, input_price_m


cost_estimator = CostEstimator()
