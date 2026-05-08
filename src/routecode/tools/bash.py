import subprocess
import os
from typing import Any, Dict, Optional, TYPE_CHECKING
from pydantic import BaseModel, Field
from .base import BaseTool

if TYPE_CHECKING:
    from ..core import RouteCodeContext


class BashInput(BaseModel):
    command: str = Field(..., description="The shell command to execute")
    timeout: int = Field(30, description="Timeout in seconds (max 120)")


class BashTool(BaseTool):
    name = "run_shell_command"
    description = "Execute a shell command and return the output"
    input_schema = BashInput
    isDestructive = True

    def prompt(self) -> str:
        return (
            "- bash: Execute shell commands. Use for building, testing, running scripts, git operations. "
            "Pass the full command string. Output is truncated at 10K chars. "
            "Timeout defaults to 30 seconds (max 120)."
        )

    def get_activity_description(self, command: str = "", **kwargs) -> str:
        c = command[:40]
        return f"Bash({c})"

    def get_tool_use_summary(self, command: str = "", **kwargs) -> str:
        return f"Ran: {command[:60]}"

    def _run(
        self,
        command: str,
        timeout: int = 30,
        ctx: Optional["RouteCodeContext"] = None,
        provider: Optional[Any] = None,
        **kwargs,
    ) -> Dict[str, Any]:
        timeout = min(timeout, 120)

        # PathGuard Integration
        if ctx and ctx.path_guard:
            # Very basic check for absolute paths in the command
            import re

            # Match strings that look like absolute paths (Unix and Windows)
            paths = re.findall(
                r"(/[a-zA-Z0-9._/-]+|[a-zA-Z]:\\[a-zA-Z0-9._\\-]+)", command
            )
            for p in paths:
                _, error = ctx.path_guard.resolve(p)
                if error:
                    return {
                        "exit_code": -1,
                        "stdout": "",
                        "stderr": f"Security Error: Command contains a path that escapes the workspace: {p}",
                    }

        try:
            result = subprocess.run(
                command, shell=True, capture_output=True, text=True, timeout=timeout
            )

            def format_long_output(text: str, prefix: str) -> str:
                if not text:
                    return text
                max_lines, max_chars = 100, 10000
                lines = text.splitlines()
                if len(lines) <= max_lines and len(text) <= max_chars:
                    return text

                dump_file = ""
                try:
                    from pathlib import Path
                    import uuid

                    tmp_dir = Path.home() / ".routecode" / "tmp" / "tool-outputs"
                    tmp_dir.mkdir(parents=True, exist_ok=True)
                    dump_file = tmp_dir / f"{prefix}_{uuid.uuid4().hex[:12]}.txt"
                    dump_file.write_text(text, encoding="utf-8")
                except Exception:
                    pass

                keep_lines = 50
                hidden = max(0, len(lines) - keep_lines)
                trunc = "\n".join(lines[-keep_lines:])
                if len(trunc) > 4000:
                    trunc = trunc[-4000:]

                msg = (
                    [f"... first {hidden} lines hidden ..."]
                    if hidden > 0
                    else ["... truncated ..."]
                )
                msg.append(trunc)
                if dump_file:
                    msg.append(f"\nOutput too long and was saved to: {dump_file}")
                return "\n".join(msg)

            stdout = format_long_output(result.stdout, "stdout")
            stderr = format_long_output(result.stderr, "stderr")

            return {
                "exit_code": result.returncode,
                "stdout": stdout,
                "stderr": stderr,
                "cwd": os.getcwd(),
            }
        except subprocess.TimeoutExpired:
            return {
                "exit_code": -1,
                "stdout": "",
                "stderr": f"Error: Command timed out after {timeout} seconds. "
                f"Try increasing timeout with bash(timeout=60).",
            }
        except Exception as e:
            return {"exit_code": -1, "stdout": "", "stderr": f"Error: {str(e)}"}
