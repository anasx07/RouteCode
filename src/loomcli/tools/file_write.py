import os
from typing import Any, Dict, Optional, TYPE_CHECKING
from pydantic import BaseModel, Field
from .base import BaseTool

if TYPE_CHECKING:
    from ..context import LoomContext
from ..utils import safe_resolve_path

class FileWriteInput(BaseModel):
    file_path: str = Field(..., description="The path to the file to create or overwrite")
    content: str = Field(..., description="The content to write to the file")

class FileWriteTool(BaseTool):
    name = "file_write"
    description = "Create a new file or overwrite an existing one with content"
    input_schema = FileWriteInput
    isDestructive = True

    def prompt(self) -> str:
        return ("- file_write: Create or overwrite a file with content. "
                "Provide file_path and content. Creates parent directories automatically. Runs serially.")

    def execute(self, file_path: str, content: str, ctx: Optional["LoomContext"] = None, 
                provider: Optional[Any] = None, **kwargs) -> Dict[str, Any]:
        resolved, error = safe_resolve_path(file_path)
        if error:
            return {"success": False, "error": error}

        try:
            os.makedirs(os.path.dirname(resolved), exist_ok=True)
            
            # Check if file exists to count old lines
            old_line_count = 0
            if os.path.exists(resolved):
                with open(resolved, "r", encoding="utf-8") as f:
                    old_line_count = len(f.readlines())

            with open(resolved, "w", encoding="utf-8") as f:
                f.write(content)

            new_line_count = len(content.splitlines())

            return {
                "success": True,
                "message": f"Successfully wrote to {file_path}",
                "stats": {"added": new_line_count, "removed": old_line_count}
            }
        except Exception as e:
            return {"success": False, "error": f"Error writing file: {str(e)}"}
