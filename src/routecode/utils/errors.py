import httpx
from typing import Dict, Optional


class ClassifiedError:
    def __init__(
        self, category: str, message: str, guidance: str, recoverable: bool = True
    ):
        self.category = category
        self.message = message
        self.guidance = guidance
        self.recoverable = recoverable

    def to_message(self) -> Dict[str, str]:
        return {
            "role": "assistant",
            "content": f"[Error: {self.message}]\n{self.guidance}",
        }


ERROR_CATEGORIES = {
    "rate_limit": ClassifiedError(
        "rate_limit",
        "Rate limited by API provider",
        "Wait a moment and try again. Consider switching to a different model with /model <name>.",
        recoverable=True,
    ),
    "timeout": ClassifiedError(
        "timeout",
        "Request timed out",
        "The request took too long. Try again with a smaller request or use /model to switch to a faster model.",
        recoverable=True,
    ),
    "auth": ClassifiedError(
        "auth",
        "Authentication failed",
        "Your API key is invalid or expired. Use /config to update it.",
        recoverable=False,
    ),
    "insufficient_quota": ClassifiedError(
        "insufficient_quota",
        "Insufficient API quota or credits",
        "Your API account may be out of credits. Check your billing at the provider's website.",
        recoverable=False,
    ),
    "model_not_found": ClassifiedError(
        "model_not_found",
        "Model not found or unavailable",
        "The model may be temporarily unavailable. Try /model to switch to a different model.",
        recoverable=True,
    ),
    "prompt_too_long": ClassifiedError(
        "prompt_too_long",
        "Prompt exceeds the model's context window",
        "The conversation is too long. Use /clear to start fresh, or try summarization.",
        recoverable=True,
    ),
    "server_error": ClassifiedError(
        "server_error",
        "API server error",
        "The provider's server encountered an error. Try again in a moment.",
        recoverable=True,
    ),
    "bad_request": ClassifiedError(
        "bad_request",
        "Invalid request",
        "The request format was invalid. This may be a bug. Try rephrasing your request.",
        recoverable=False,
    ),
    "connection": ClassifiedError(
        "connection",
        "Connection error",
        "Could not connect to the API provider. Check your internet connection.",
        recoverable=True,
    ),
    "unknown": ClassifiedError(
        "unknown",
        "An unexpected error occurred",
        "Try again or use /clear to reset the conversation.",
        recoverable=True,
    ),
}


# Shared keyword-to-category matching — single source of truth for both
# classify_http_error() and classify_exception().
_KEYWORD_MATCHERS = [
    ("auth", ["api key", "unauthorized", "unauth", "auth failed", "401", "403"]),
    ("insufficient_quota", ["credit", "quota", "balance", "402", "insufficient_quota"]),
    ("rate_limit", ["rate limit", "rate_limit", "429", "too many requests"]),
    ("timeout", ["timeout", "timed out", "408"]),
    ("prompt_too_long", ["too long", "context length", "maximum context", "413"]),
    ("connection", ["connect", "connection", "dns", "name resolution"]),
]


def _match_keywords(text: str) -> Optional[str]:
    """Checks a lowercase text string against ERROR_CATEGORY keyword matchers.
    Returns the category key if a match is found, or None."""
    for category, keywords in _KEYWORD_MATCHERS:
        for kw in keywords:
            if kw in text:
                return category
    return None


def _model_not_found_in(text: str) -> bool:
    return "model" in text and ("not found" in text or "not support" in text)


def classify_http_error(status_code: int, body: str = "") -> ClassifiedError:
    body_lower = body.lower()

    if status_code == 429:
        return ERROR_CATEGORIES["rate_limit"]
    elif status_code == 401 or status_code == 403:
        return ERROR_CATEGORIES["auth"]
    elif status_code == 402:
        return ERROR_CATEGORIES["insufficient_quota"]
    elif status_code == 404:
        if _model_not_found_in(body_lower):
            return ERROR_CATEGORIES["model_not_found"]
        return ERROR_CATEGORIES["bad_request"]
    elif status_code == 413:
        return ERROR_CATEGORIES["prompt_too_long"]
    elif status_code == 408:
        return ERROR_CATEGORIES["timeout"]
    elif status_code >= 500:
        return ERROR_CATEGORIES["server_error"]
    elif status_code >= 400:
        cat = _match_keywords(body_lower)
        if cat:
            return ERROR_CATEGORIES[cat]
        return ERROR_CATEGORIES["bad_request"]
    else:
        return ERROR_CATEGORIES["unknown"]


def classify_exception(e: Exception) -> ClassifiedError:
    if isinstance(e, httpx.TimeoutException):
        return ERROR_CATEGORIES["timeout"]
    elif isinstance(e, httpx.ConnectError):
        return ERROR_CATEGORIES["connection"]
    elif isinstance(e, httpx.HTTPStatusError):
        try:
            body = e.response.text if hasattr(e.response, "text") else ""
        except Exception:
            body = ""
        return classify_http_error(e.response.status_code, body)
    elif isinstance(e, httpx.RemoteProtocolError):
        return ERROR_CATEGORIES["connection"]

    msg = str(e).lower()
    if _model_not_found_in(msg):
        return ERROR_CATEGORIES["model_not_found"]

    cat = _match_keywords(msg)
    if cat:
        return ERROR_CATEGORIES[cat]

    return ERROR_CATEGORIES["unknown"]
