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

    # Clear existing handlers if any
    root = logging.getLogger()
    if root.handlers:
        for handler in root.handlers[:]:
            root.removeHandler(handler)

    logging.basicConfig(
        level=level,
        format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
        handlers=[logging.FileHandler(LOG_FILE, encoding="utf-8")],
    )

    # Suppress verbose third-party logs
    logging.getLogger("httpx").setLevel(logging.WARNING)
    logging.getLogger("httpcore").setLevel(logging.WARNING)
    logging.getLogger("prompt_toolkit").setLevel(logging.ERROR)

    logging.info("Logging initialized. File: %s", LOG_FILE)


def get_logger(name: str):
    """Returns a logger for a specific module."""
    return logging.getLogger(f"routecode.{name}")
