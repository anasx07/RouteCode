import difflib
import os
from typing import Any, Dict, List, Optional, TYPE_CHECKING
from pydantic import BaseModel, Field
from .base import BaseTool

if TYPE_CHECKING:
    from ..context import LoomContext
from ..utils import safe_resolve_path

class FileEditInput(BaseModel):
    file_path: str = Field(..., description="The path to the file to edit")
    old_string: str = Field(..., description="The exact string to be replaced")
    new_string: str = Field(..., description="The string to replace old_string with")
    allow_multiple: bool = Field(False, description="If True, replace all occurrences of old_string. If False, only succeed if exactly one occurrence is found.")

class FileEditTool(BaseTool):
    name = "file_edit"
    description = "Surgically edit a file by replacing specific strings"
    input_schema = FileEditInput
    isDestructive = True

    def prompt(self) -> str:
        return ("- file_edit: Make surgical string replacements in files. "
                "Provide old_string (exact match) and new_string. "
                "Set allow_multiple=True to replace all occurrences. Runs serially.")

    def execute(self, file_path: str, old_string: str, new_string: str, allow_multiple: bool = False, 
                ctx: Optional["LoomContext"] = None, provider: Optional[Any] = None, **kwargs) -> Dict[str, Any]:
        resolved, error = safe_resolve_path(file_path)
        if error:
            return {"success": False, "error": error}
        if not os.path.exists(resolved):
            return {"success": False, "error": f"File not found: {file_path}"}

        try:
            with open(resolved, "r", encoding="utf-8") as f:
                content = f.read()

            count = content.count(old_string)
            if count == 0:
                return {"success": False, "error": "The 'old_string' was not found in the file. Ensure it matches exactly."}

            if not allow_multiple and count > 1:
                return {"success": False, "error": f"The 'old_string' was found {count} times. Please provide more context to make it unique, or set allow_multiple=True."}

            new_content = content.replace(old_string, new_string)

            with open(resolved, "w", encoding="utf-8") as f:
                f.write(new_content)

            added = new_string.count('\n') + (1 if new_string and not new_string.endswith('\n') else 0)
            removed = old_string.count('\n') + (1 if old_string and not old_string.endswith('\n') else 0)

            old_lines = old_string.splitlines(keepends=True)
            new_lines = new_string.splitlines(keepends=True)
            diff_lines = list(difflib.unified_diff(
                old_lines, new_lines,
                fromfile=resolved, tofile=resolved,
                lineterm=''
            ))

            return {
                "success": True,
                "message": f"Successfully edited {file_path}. Replaced {count} occurrence(s).",
                "stats": {"added": added * count, "removed": removed * count},
                "diff": diff_lines
            }
        except Exception as e:
            return {"success": False, "error": f"Error during file edit: {str(e)}"}
