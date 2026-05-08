from typing import Any, Callable, Dict, Optional, TYPE_CHECKING
from fnmatch import fnmatch

from .base import ToolMiddleware, BaseTool

if TYPE_CHECKING:
    from ..core import RouteCodeContext


class AuthorizationMiddleware(ToolMiddleware):
    """
    Middleware that enforces security policies (allow/deny lists)
    and handles interactive confirmation for destructive tools.
    """

    def __init__(self, confirm_callback: Optional[Callable] = None):
        self.confirm_callback = confirm_callback

    async def __call__(
        self,
        tool: BaseTool,
        args: Dict[str, Any],
        ctx: "RouteCodeContext",
        next_call: Callable,
    ) -> Any:
        if not tool.isDestructive:
            return await next_call(tool, args, ctx)

        # Security: Auto-allow "safe" read-only bash commands
        if tool.name == "bash":
            cmd = args.get("command", "").strip().lower()
            safe_prefixes = [
                "ls",
                "cat",
                "pwd",
                "dir",
                "echo",
                "git status",
                "git diff",
                "type",
            ]
            # Check if command starts with a safe prefix or is just the safe command
            if any(cmd == p or cmd.startswith(p + " ") for p in safe_prefixes):
                # Also ensure no pipe or redirection to a file (to be safe)
                if ">" not in cmd and "|" not in cmd:
                    return await next_call(tool, args, ctx)

        # Check Persistent Allowlist
        if self._check_permission(tool.name, ctx.config.allowlist):
            return await next_call(tool, args, ctx)

        # Check Session Allowlist
        if self._check_permission(tool.name, ctx.state.session_allowlist):
            return await next_call(tool, args, ctx)

        # Check Denylist
        if self._check_permission(tool.name, ctx.config.denylist):
            return {"error": f"Tool execution blocked by denylist: {tool.name}"}

        # Interactive confirmation
        if self.confirm_callback:
            allowed = await self.confirm_callback(tool, args)
            if not allowed:
                return {"error": "Permission denied by user"}
            return await next_call(tool, args, ctx)

        # Fallback if no confirmation callback is available for a destructive tool
        return {
            "error": f"Interactive confirmation required for destructive tool: {tool.name}"
        }

    def _check_permission(self, tool_name: str, pattern_list: Optional[list]) -> bool:
        if not pattern_list:
            return False

        pattern = f"{tool_name}(*)"
        for rule in pattern_list:
            if fnmatch(pattern, rule):
                return True
        return False
