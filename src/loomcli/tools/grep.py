import os
import re
from typing import Any, Dict, List, Optional, TYPE_CHECKING
from pydantic import BaseModel, Field
from .base import BaseTool

if TYPE_CHECKING:
    from ..core import LoomContext

class GrepInput(BaseModel):
    pattern: str = Field(..., description="Regex pattern to search for in file contents")
    include: Optional[str] = Field(None, description="Glob pattern to filter files (e.g., '*.py', '*.{ts,tsx}')")
    path: Optional[str] = Field(None, description="Directory to search in (defaults to current working directory)")

class GrepTool(BaseTool):
    name = "grep"
    description = "Search file contents for a regex pattern, returning matching files with line numbers"
    input_schema = GrepInput
    isConcurrencySafe = True
    isReadOnly = True

    def prompt(self) -> str:
        return ("- grep: Search file contents with regex patterns. "
                "Use include='*.py' to filter by extension. Capped at 50 files. Safe for concurrent use.")

    def execute(self, pattern: str, include: Optional[str] = None, path: Optional[str] = None, 
                ctx: Optional["LoomContext"] = None, provider: Optional[Any] = None, **kwargs) -> Dict[str, Any]:
        import glob as glob_module

        try:
            search_root = path or os.getcwd()
            if not os.path.isdir(search_root):
                return {"success": False, "error": f"Directory not found: {search_root}"}

            compiled = re.compile(pattern, re.IGNORECASE)

            if include:
                glob_pattern = os.path.join(search_root, "**", include)
                files = glob_module.glob(glob_pattern, recursive=True)
            else:
                files = []
                for root, dirs, fnames in os.walk(search_root):
                    for fname in fnames:
                        fp = os.path.join(root, fname)
                        files.append(fp)

            files = [f for f in files if os.path.isfile(f)]

            text_extensions = {".py", ".ts", ".tsx", ".js", ".jsx", ".rs", ".go", ".java",
                               ".c", ".cpp", ".h", ".hpp", ".cs", ".rb", ".php", ".swift",
                               ".kt", ".scala", ".md", ".txt", ".json", ".yaml", ".yml",
                               ".toml", ".cfg", ".ini", ".xml", ".html", ".css", ".scss",
                               ".sql", ".sh", ".bat", ".ps1", ".env", ".gitignore", ".lock"}

            results = []
            max_results = 50
            max_lines_per_file = 5

            for fp in files:
                if len(results) >= max_results:
                    break

                ext = os.path.splitext(fp)[1].lower()
                if ext and ext not in text_extensions:
                    continue

                try:
                    with open(fp, "r", encoding="utf-8", errors="replace") as f:
                        matches_in_file = []
                        for i, line in enumerate(f, 1):
                            if compiled.search(line):
                                matches_in_file.append((i, line.rstrip()))
                                if len(matches_in_file) >= max_lines_per_file:
                                    break

                        if matches_in_file:
                            results.append({
                                "file": fp,
                                "matches": [{"line": ln, "content": lc} for ln, lc in matches_in_file],
                                "match_count": len(matches_in_file)
                            })
                except Exception:
                    continue

            return {
                "success": True,
                "results": results,
                "num_results": len(results),
                "truncated": len(results) >= max_results
            }
        except Exception as e:
            return {"success": False, "error": f"Error during grep: {str(e)}"}
