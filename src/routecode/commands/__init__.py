from typing import Dict
import inspect
from ..core import RouteCodeContext
from .core import (
    handle_help,
    handle_tools,
    handle_attach,
    handle_version,
    handle_exit,
    handle_update,
)
from .config import (
    handle_provider,
    handle_model,
    handle_config,
    handle_theme,
    handle_personality,
)
from .session import (
    handle_history,
    handle_save,
    handle_load,
    handle_rewind,
    handle_new,
    handle_edit,
    handle_search,
)
from .tasks import handle_tasks, handle_task_stop
from .memory import handle_remember, handle_forget, handle_memories

COMMANDS = {
    "/help": handle_help,
    "/tools": handle_tools,
    "/provider": handle_provider,
    "/model": handle_model,
    "/personality": handle_personality,
    "/theme": handle_theme,
    "/config": handle_config,
    "/history": handle_history,
    "/save": handle_save,
    "/load": handle_load,
    "/new": handle_new,
    "/tasks": handle_tasks,
    "/task-stop": handle_task_stop,
    "/rewind": handle_rewind,
    "/edit": handle_edit,
    "/search": handle_search,
    "/attach": handle_attach,
    "/version": handle_version,
    "/remember": handle_remember,
    "/forget": handle_forget,
    "/memories": handle_memories,
    "/exit": handle_exit,
    "/update": handle_update,
}


def get_command_metadata() -> Dict[str, str]:
    return {
        "/help": "Show help menu",
        "/tools": "List available agent tools",
        "/provider": "Select AI provider interactively",
        "/model": "Get or set the active model",
        "/personality": "Set the AI personality/style",
        "/theme": "Change the color theme",
        "/config": "Manage keys (Update, Delete, View)",
        "/history": "Show recent conversation messages",
        "/save [name]": "Save the current conversation",
        "/load": "Load a saved conversation",
        "/new": "Start a new session (clears history & counters)",
        "/tasks": "List active and recent tasks",
        "/task-stop <id>": "Stop a running task",
        "/rewind [n]": "Remove the last N turns from the conversation",
        "/edit <idx>": "Edit a past message in the conversation",
        "/search <term>": "Search conversation history for a term",
        "/attach <file>": "Attach an image, text file, or PDF to the conversation",
        "/version": "Show version info",
        "/remember <key> <value>": "Save a memory for future sessions",
        "/forget <key>": "Delete a saved memory",
        "/memories": "List all saved memories",
        "/exit": "Exit the session",
        "/update": "Check for and install RouteCode updates",
    }


async def execute_command(input_str: str, ctx: RouteCodeContext) -> bool:
    from ..domain.skills import discover_skills

    parts = input_str.split()
    if not parts:
        return False
    command = parts[0]
    args = parts[1:]

    if command in COMMANDS:
        ctx.state.commands_run += 1
        handler = COMMANDS[command]
        if inspect.iscoroutinefunction(handler):
            await handler(args, ctx)
        else:
            handler(args, ctx)
        return True

    # Check for skill-based commands
    skills = discover_skills()
    skill_name = command.lstrip("/")
    if skill_name in skills:
        skill = skills[skill_name]
        arg_str = " ".join(args)
        label = f"Skill({skill.name})"
        ctx.state.commands_run += 1
        ctx.console.print_tool_call(label, {})
        from ..domain.skills import run_skill

        # Skills might be sync or async, but for now they are mostly sync
        result = run_skill(skill, arg_str)
        ctx.console.print_tool_result(result)
        return True

    return False
