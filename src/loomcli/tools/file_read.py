import os
import difflib
from typing import Any, Dict, List, Optional, TYPE_CHECKING
from pydantic import BaseModel, Field
from .base import BaseTool

if TYPE_CHECKING:
    from ..core import LoomContext
from ..utils import safe_resolve_path


def _find_similar_file(path: str) -> List[str]:
    """Suggest similar filenames when a file isn't found."""
    head, tail = os.path.split(path)
    if not os.path.isdir(head):
        head = os.getcwd()
    if not os.path.isdir(head):
        return []
    try:
        candidates = []
        for f in os.listdir(head):
            ratio = difflib.SequenceMatcher(None, tail.lower(), f.lower()).ratio()
            if ratio > 0.4:
                candidates.append((ratio, f))
        candidates.sort(reverse=True)
        return [f for _, f in candidates[:3]]
    except Exception:
        return []


def add_line_numbers(content: str, start_line: int = 1) -> str:
    lines = content.split("\n")
    result = []
    for i, line in enumerate(lines, start=start_line):
        result.append(f"{i:6}\t{line}")
    return "\n".join(result)


class FileReadInput(BaseModel):
    file_path: str = Field(..., description="The path to the file to read")
    offset: int = Field(0, description="Line number to start reading from (1-indexed, default 0 = start of file)")
    limit: int = Field(0, description="Maximum number of lines to read (default 0 = read entire file)")

class FileReadTool(BaseTool):
    name = "file_read"
    description = "Read the content of a file"
    input_schema = FileReadInput
    isConcurrencySafe = True
    isReadOnly = True

    def prompt(self) -> str:
        return ("- file_read: Reads a file from the filesystem. Results are returned "
                "with cat -n format with line numbers. Use offset/limit for large files. "
                "Can read text files, images (PNG, JPG, etc.), and notebooks.")

    def get_activity_description(self, file_path: str = "", **kwargs) -> str:
        return f"Read({file_path})"

    def execute(self, file_path: str, offset: int = 0, limit: int = 0, 
                ctx: Optional["LoomContext"] = None, provider: Optional[Any] = None, **kwargs) -> Dict[str, Any]:
        resolved, error = safe_resolve_path(file_path)
        if error:
            return {"success": False, "error": error}
        if not os.path.exists(resolved):
            suggestions = _find_similar_file(resolved)
            msg = f"File not found: {file_path}"
            if suggestions:
                msg += f" Did you mean: {', '.join(suggestions)}?"
            return {"success": False, "error": msg}

        try:
            with open(resolved, "r", encoding="utf-8") as f:
                lines = f.readlines()

            total_lines = len(lines)
            start = max(0, offset - 1) if offset > 0 else 0
            end = start + limit if limit > 0 else total_lines
            selected = lines[start:end]

            raw_content = "".join(selected)
            numbered_content = add_line_numbers(raw_content, start + 1)

            info = f"Showing lines {start + 1}-{min(end, total_lines)} of {total_lines}"
            if total_lines > end:
                info += f". Use offset={end + 1} to read more."

            return {
                "success": True,
                "content": raw_content,
                "numbered_content": numbered_content,
                "total_lines": total_lines,
                "info": info
            }
        except Exception as e:
            return {"success": False, "error": f"Error reading file: {str(e)}"}
