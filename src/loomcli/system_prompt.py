import os
import subprocess
from typing import Optional
from .tools import registry


SYSTEM_PROMPT_DYNAMIC_BOUNDARY = "__DYNAMIC__"


def _build_identity_section() -> str:
    return """You are Loomb CLI, an AI software engineering assistant running in a terminal.
You have access to local file system tools and a shell environment.
When responding, start with a <thought> block for your internal reasoning and plan, then provide your response."""


def _build_tools_section() -> str:
    tool_prompts = []
    for name, tool in registry.list_tools_with_prompts().items():
        tool_prompts.append(tool)
    if tool_prompts:
        return "## Available Tools\n" + "\n".join(tool_prompts)
    return ""


def _build_behavior_section() -> str:
    return """## Behavior
- Be concise. Use tools when needed, not for every response.
- Prefer `file_read` over `bash cat` for reading files. file_read returns results in cat -n format with line numbers.
- Use `glob` and `grep` for codebase exploration before editing.
- When editing, use `file_edit` for surgical changes, `file_write` for new files.
- Use `task` for complex multi-step work that requires autonomous execution.
- ALWAYS use file_read to view files before editing them.

## Parallel Execution
The following tools are concurrency-safe and can be run in parallel:
- `file_read`, `glob`, `grep`, `webfetch` — read-only and safe to batch.

Tools like `bash`, `file_edit`, `file_write` run serially (one at a time).
When you have multiple read operations, batch them together in one response."""


def _build_memory_section() -> str:
    from .memory import memory_manager
    return memory_manager.get_relevant_context()


def _build_loom_section() -> str:
    if os.path.exists("LOOM.md"):
        try:
            content = open("LOOM.md", encoding="utf-8").read()
            if "## System Prompt" in content:
                section = content.split("## System Prompt", 1)[1]
                if "## " in section:
                    section = section.split("## ", 1)[0]
                return section.strip()
            return "## Project Instructions\n" + content
        except Exception:
            return None
    return None


def _build_context_section() -> str:
    parts = []
    for filename in ["README.md", "pyproject.toml"]:
        if os.path.exists(filename):
            try:
                with open(filename, "r", encoding="utf-8") as f:
                    parts.append(f"--- {filename} ---\n{f.read()}")
            except Exception:
                pass
    if parts:
        return "## Project Context\n" + "\n".join(parts)
    return ""


def _build_git_section() -> str:
    from .git import get_git_context
    return get_git_context()


def _build_skill_section() -> str:
    from .skills import get_skill_prompts
    return get_skill_prompts()


def _build_env_section() -> str:
    import getpass, platform
    user = getpass.getuser()
    plat = platform.platform()
    cwd = os.getcwd()
    is_git = os.path.isdir(os.path.join(cwd, ".git")) if cwd else False
    return f"""<env>
Working directory: {cwd}
Is directory a git repo: {'Yes' if is_git else 'No'}
Platform: {plat}
User: {user}
</env>"""


def compute_system_prompt() -> str:
    from .personalities import get_personality_section, get_active_personality

    p = get_active_personality()
    sections = [
        _build_identity_section(),
    ]
    if p.keep_base_instructions:
        sections.append(_build_behavior_section())
    sections += [
        _build_tools_section(),
        _build_skill_section(),
        _build_env_section(),
        _build_loom_section(),
        _build_memory_section(),
        _build_git_section(),
        get_personality_section(),
        SYSTEM_PROMPT_DYNAMIC_BOUNDARY,
        _build_context_section(),
    ]
    return "\n\n".join(s for s in sections if s)
