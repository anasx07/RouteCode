import glob as glob_module
import os
from typing import Any, Dict, List, Optional, TYPE_CHECKING
from pydantic import BaseModel, Field
from .base import BaseTool

if TYPE_CHECKING:
    from ..context import LoomContext

class GlobInput(BaseModel):
    pattern: str = Field(..., description="Glob pattern to match (e.g., '**/*.py', 'src/**/*.ts')")
    path: Optional[str] = Field(None, description="Directory to search in (defaults to current working directory)")

class GlobTool(BaseTool):
    name = "glob"
    description = "Search for files matching a glob pattern, sorted by modification time (most recent first)"
    input_schema = GlobInput
    isConcurrencySafe = True
    isReadOnly = True

    def prompt(self) -> str:
        return ("- glob: Search for files by glob pattern (e.g., '**/*.py'). "
                "Results sorted by modification time, capped at 100. Safe for concurrent use.")

    def execute(self, pattern: str, path: Optional[str] = None, ctx: Optional["LoomContext"] = None) -> Dict[str, Any]:
        try:
            search_root = path or os.getcwd()
            if not os.path.isdir(search_root):
                return {"success": False, "error": f"Directory not found: {search_root}"}

            full_pattern = os.path.join(search_root, pattern)
            matches = glob_module.glob(full_pattern, recursive=True)
            matches = [m for m in matches if os.path.isfile(m)]
            matches.sort(key=lambda p: os.path.getmtime(p), reverse=True)

            if not matches:
                return {"success": True, "files": [], "message": "No files matched the pattern."}

            max_results = 100
            truncated = len(matches) > max_results
            files = matches[:max_results]

            return {
                "success": True,
                "files": files,
                "num_results": len(files),
                "truncated": truncated,
                "total_matches": len(matches)
            }
        except Exception as e:
            return {"success": False, "error": f"Error during glob: {str(e)}"}
