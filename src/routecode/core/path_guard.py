import os
from typing import Tuple, Optional


class PathGuard:
    """
    Centralized service for safe path resolution and workspace sandboxing.
    Dynamic: Looks up the current working directory at call-time.
    """

    def __init__(self, root: Optional[str] = None):
        self._root = root

    def get_workspace(self) -> str:
        """Returns the canonical workspace root."""
        if self._root:
            return os.path.realpath(self._root)
        return os.path.realpath(os.getcwd())

    def resolve(self, path: str) -> Tuple[Optional[str], Optional[str]]:
        """
        Resolves a path relative to the current workspace and validates sandboxing.
        Returns (resolved_absolute_path, error_message).

        Guards against:
          - Absolute path traversal (e.g. /etc/passwd)
          - Prefix attacks (e.g. /my_dir vs /my_dir_secret)
          - Symlink escape (e.g. workspace/link -> /etc/passwd)
        """
        ws = self.get_workspace()
        try:
            joined = os.path.join(ws, path) if not os.path.isabs(path) else path
            resolved = os.path.abspath(os.path.normpath(joined))
            resolved = os.path.realpath(resolved)
        except (ValueError, OSError):
            return None, f"Invalid path format: {path}"

        # Canonicalize workspace root too, so symlinks in the root are also resolved
        ws_canonical = os.path.realpath(ws)

        # Trailing-separator check prevents prefix attacks
        ws_sep = ws_canonical if ws_canonical.endswith(os.sep) else ws_canonical + os.sep
        res_sep = resolved if resolved.endswith(os.sep) else resolved + os.sep

        if not res_sep.startswith(ws_sep) and resolved != ws_canonical:
            return None, f"Path escapes workspace sandbox: {path}"

        return resolved, None
