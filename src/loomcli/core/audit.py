import json
import logging
from logging.handlers import RotatingFileHandler
from typing import Any, Dict
from ..config import CONFIG_DIR


class AuditLogger:
    """
    Persistent, rotating audit log for tracking all tool executions.
    Focuses on security-sensitive and destructive operations.
    """

    def __init__(self):
        log_dir = CONFIG_DIR / "logs"
        log_dir.mkdir(parents=True, exist_ok=True)
        self.log_file = log_dir / "audit.log"

        self.logger = logging.getLogger("loom.audit")
        self.logger.setLevel(logging.INFO)

        # 5MB per file, keep 5 rotations
        handler = RotatingFileHandler(
            self.log_file, maxBytes=5 * 1024 * 1024, backupCount=5
        )
        formatter = logging.Formatter("%(asctime)s - %(message)s")
        handler.setFormatter(formatter)
        self.logger.addHandler(handler)

    def log_tool_result(self, tool_name: str, result: Dict[str, Any]):
        """Logs the completion of a tool call."""
        status = "SUCCESS" if result.get("success", True) else "FAILED"
        if "error" in result:
            status = f"FAILED: {result['error']}"

        # Truncate large results for the log
        log_data = {
            "tool": tool_name,
            "status": status,
            "stats": result.get("stats", {}),
            "message": result.get("message", "")[:200],
        }

        self.logger.info(json.dumps(log_data))


audit_logger = AuditLogger()


def audit_hook(name: str, result: Dict[str, Any]):
    """Hook function to be registered with ToolRegistry."""
    audit_logger.log_tool_result(name, result)
