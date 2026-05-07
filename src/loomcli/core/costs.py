import litellm
import re
from typing import Tuple
from .logger import get_logger
from .models_db import get_model_pricing

logger = get_logger(__name__)


class CostEstimator:
    """
    Decoupled service for calculating token counts and financial costs.
    Leverages litellm for robust tokenization and pricing data.
    """

    def __init__(
        self, default_input_price: float = 2.0, default_output_price: float = 10.0
    ):
        # Default prices per 1M tokens
        self.default_input_price = default_input_price
        self.default_output_price = default_output_price

    def count_tokens(self, text: str, model: str) -> int:
        """
        Calculates token count using litellm's token_counter,
        which uses the appropriate tokenizer for the given model.
        """
        try:
            # LiteLLM handles different tokenizers (tiktoken, anthropic, etc.)
            return litellm.token_counter(model=model, text=text)
        except Exception as e:
            logger.debug(
                "LiteLLM token_counter failed for %s: %s. Using regex fallback.",
                model,
                e,
            )
            # Last resort fallback if litellm fails or model is unknown
            # Provides a better estimate for code/JSON than simple split()
            tokens = re.findall(r"\w+|[^\w\s]", text)
            return len(tokens)

    def calculate_cost(
        self, input_tokens: int, output_tokens: int, model: str
    ) -> Tuple[float, int, float]:
        """
        Calculates the cost based on input and output tokens for a specific model.
        Returns (estimated_cost, context_limit, input_cost_per_1m).
        """
        # Try to get pricing from litellm first
        try:
            model_info = litellm.get_model_info(model)
            if model_info:
                input_price = model_info.get("input_cost_per_token")
                output_price = model_info.get("output_cost_per_token")
                context_limit = model_info.get("max_tokens", 32000)

                if input_price is not None and output_price is not None:
                    cost = (input_tokens * input_price) + (output_tokens * output_price)
                    # Normalize to per 1M tokens for the third return value
                    return cost, context_limit, input_price * 1_000_000
        except Exception as e:
            logger.debug("Failed to get model info from LiteLLM for %s: %s", model, e)

        # Fallback to internal models_db
        input_price_m, output_price_m, context_limit = get_model_pricing(model)
        # models_db prices are per 1M tokens
        cost = (input_tokens * input_price_m / 1_000_000) + (
            output_tokens * output_price_m / 1_000_000
        )
        return cost, context_limit, input_price_m


cost_estimator = CostEstimator()
