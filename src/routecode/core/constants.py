"""
Centralized constants for the RouteCode application.
All magic numbers that control limits, truncation, and capacity
should be defined here with clear documentation.
"""

# ── Tool result & file attachment limits ────────────────────────────────
MAX_TOOL_RESULT_CHARS = 50000
MAX_ATTACHMENT_CHARS = 50000
MAX_FETCH_CHARS = 50000

# ── Memory limits ───────────────────────────────────────────────────────
MAX_MEMORIES = 50
MAX_MEMORY_CHARS = 500

# ── UI / Config limits ──────────────────────────────────────────────────
MAX_RECENT_MODELS = 10

# ── Task / Orchestrator limits ──────────────────────────────────────────
MAX_TASK_HISTORY = 50
MAX_ORCHESTRATOR_TURNS = 20
SUMMARY_KEEP_COUNT = 7
