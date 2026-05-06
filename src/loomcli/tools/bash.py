import subprocess
import os
import time
import threading
from typing import Any, Dict, Optional, TYPE_CHECKING
from pydantic import BaseModel, Field
from .base import BaseTool

if TYPE_CHECKING:
    from ..context import LoomContext

class BashInput(BaseModel):
    command: str = Field(..., description="The shell command to execute")
    timeout: int = Field(30, description="Timeout in seconds (max 120)")

class BashTool(BaseTool):
    name = "bash"
    description = "Execute a shell command and return the output"
    input_schema = BashInput
    isDestructive = True

    def prompt(self) -> str:
        return ("- bash: Execute shell commands. Use for building, testing, running scripts, git operations. "
                "Pass the full command string. Output is truncated at 10K chars. "
                "Timeout defaults to 30 seconds (max 120).")

    def get_activity_description(self, command: str = "", **kwargs) -> str:
        c = command[:40]
        return f"Bash({c})"

    def get_tool_use_summary(self, command: str = "", **kwargs) -> str:
        return f"Ran: {command[:60]}"

    def execute(self, command: str, timeout: int = 30, ctx: Optional["LoomContext"] = None, 
                provider: Optional[Any] = None, **kwargs) -> Dict[str, Any]:
        import sys
        timeout = min(timeout, 120)
        try:
            result = subprocess.run(
                command,
                shell=True,
                capture_output=True,
                text=True,
                timeout=timeout
            )
            
            stdout = result.stdout
            stderr = result.stderr
            
            max_len = 10000
            if len(stdout) > max_len:
                stdout = stdout[:max_len] + "... [Output truncated]"
            if len(stderr) > max_len:
                stderr = stderr[:max_len] + "... [Output truncated]"
                
            return {
                "exit_code": result.returncode,
                "stdout": stdout,
                "stderr": stderr,
                "cwd": os.getcwd()
            }
        except subprocess.TimeoutExpired:
            return {
                "exit_code": -1,
                "stdout": "",
                "stderr": f"Error: Command timed out after {timeout} seconds. "
                         f"Try increasing timeout with bash(timeout=60)."
            }
        except Exception as e:
            return {
                "exit_code": -1,
                "stdout": "",
                "stderr": f"Error: {str(e)}"
            }
