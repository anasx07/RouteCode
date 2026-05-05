import os
from typing import Optional

DEFAULT_WORKSPACE = os.path.abspath(os.getcwd())


def safe_resolve_path(file_path: str, workspace: Optional[str] = None) -> tuple:
    """Resolve a file path and validate it's within the workspace.
    
    Returns (resolved_absolute_path, error_message).
    On success, error_message is None.
    On failure, resolved_absolute_path is None.
    """
    ws = os.path.abspath(workspace or DEFAULT_WORKSPACE)
    try:
        joined = os.path.join(ws, file_path) if not os.path.isabs(file_path) else file_path
        resolved = os.path.abspath(os.path.normpath(joined))
        resolved = os.path.realpath(resolved)
    except (ValueError, OSError):
        return None, f"Invalid path: {file_path}"

    if not resolved.startswith(ws):
        return None, f"Path escapes workspace ({ws}): {file_path}"

    return resolved, None


def is_text_file(file_path: str) -> bool:
    _, ext = os.path.splitext(file_path)
    text_extensions = {".py", ".ts", ".tsx", ".js", ".jsx", ".rs", ".go", ".java",
                       ".c", ".cpp", ".h", ".hpp", ".cs", ".rb", ".php", ".swift",
                       ".kt", ".md", ".txt", ".json", ".yaml", ".yml",
                       ".toml", ".cfg", ".ini", ".xml", ".html", ".css", ".scss",
                       ".sql", ".sh", ".bat", ".ps1", ".env", ".gitignore", ".lock",
                       ".pyw", ".r", ".m", ".mm", ".pl", ".pm", ".lua", ".scala",
                       ".clj", ".ex", ".exs", ".hs", ".nim", ".zig", ".cjs", ".mjs"}
    return ext.lower() in text_extensions or not ext
