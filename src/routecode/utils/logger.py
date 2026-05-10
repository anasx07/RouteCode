import logging
from ..config import CONFIG_DIR

LOG_FILE = CONFIG_DIR / "routecode.log"


def setup_logging(level=logging.INFO):
    """
    Configures structured logging for routecode.
    Internal diagnostics go to a file, while user-facing output remains via Rich.
    """
    if not CONFIG_DIR.exists():
        CONFIG_DIR.mkdir(parents=True, exist_ok=True)

    root = logging.getLogger()
    if root.handlers:
        for handler in root.handlers[:]:
            root.removeHandler(handler)

    numeric_level = getattr(logging, level.upper()) if isinstance(level, str) else level

    logging.basicConfig(
        level=numeric_level,
        format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
        handlers=[logging.FileHandler(LOG_FILE, encoding="utf-8")],
    )

    logging.getLogger("httpx").setLevel(logging.WARNING)
    logging.getLogger("httpcore").setLevel(logging.WARNING)
    logging.getLogger("prompt_toolkit").setLevel(logging.ERROR)

    logging.info("Logging initialized. File: %s  Level: %s", LOG_FILE, level)


def get_logger(name: str):
    """Returns a logger for a specific module."""
    return logging.getLogger(f"routecode.{name}")
