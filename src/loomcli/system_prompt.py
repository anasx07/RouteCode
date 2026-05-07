import os
import asyncio
from typing import Optional, TYPE_CHECKING
from .tools import registry

if TYPE_CHECKING:
    from .core import LoomContext


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


def _build_memory_section(ctx: "LoomContext") -> str:
    return ctx.memory.get_relevant_context()


_prompt_file_cache: dict = {}
_prompt_file_mtimes: dict = {}


async def _get_file_content_cached(filename: str) -> Optional[str]:
    """Helper to read file content with MTIME caching."""
    if not os.path.exists(filename):
        return None

    try:
        mtime = os.stat(filename).st_mtime
        if filename in _prompt_file_cache and mtime <= _prompt_file_mtimes.get(
            filename, 0
        ):
            return _prompt_file_cache[filename]

        import aiofiles

        async with aiofiles.open(filename, mode="r", encoding="utf-8") as f:
            content = await f.read()
            _prompt_file_cache[filename] = content
            _prompt_file_mtimes[filename] = mtime
            return content
    except Exception:
        return None


async def _build_loom_section_async() -> Optional[str]:
    content = await _get_file_content_cached("LOOM.md")
    if content:
        if "## System Prompt" in content:
            section = content.split("## System Prompt", 1)[1]
            if "## " in section:
                section = section.split("## ", 1)[0]
            return section.strip()
        return "## Project Instructions\n" + content
    return None


async def _build_context_section_async() -> str:
    parts = []
    for filename in ["README.md", "pyproject.toml"]:
        content = await _get_file_content_cached(filename)
        if content:
            parts.append(f"--- {filename} ---\n{content}")
    if parts:
        return "## Project Context\n" + "\n".join(parts)
    return ""


async def _build_git_section_async() -> str:
    from .git import get_git_context_async

    return await get_git_context_async()


def _build_skill_section() -> str:
    from .skills import get_skill_prompts

    return get_skill_prompts()


def _build_env_section() -> str:
    import getpass
    import platform

    user = getpass.getuser()
    plat = platform.platform()
    cwd = os.getcwd()
    is_git = os.path.isdir(os.path.join(cwd, ".git")) if cwd else False
    return f"""<env>
Working directory: {cwd}
Is directory a git repo: {"Yes" if is_git else "No"}
Platform: {plat}
User: {user}
</env>"""


async def compute_system_prompt(ctx: "LoomContext") -> str:
    from .personalities import get_personality_section, get_active_personality

    if ctx.config.personality:
        from .personalities import load_personalities

        pers = load_personalities().get(ctx.config.personality)
    else:
        pers = get_active_personality()

    sections = [
        _build_identity_section(),
    ]
    if pers.keep_base_instructions:
        sections.append(_build_behavior_section())

    # Gather dynamic sections in parallel
    dynamic_results = await asyncio.gather(
        _build_loom_section_async(),
        _build_git_section_async(),
        _build_context_section_async(),
    )

    loom_sect, git_sect, context_sect = dynamic_results

    sections += [
        _build_tools_section(),
        _build_skill_section(),
        _build_env_section(),
        loom_sect,
        _build_memory_section(ctx),
        git_sect,
        get_personality_section(),
        SYSTEM_PROMPT_DYNAMIC_BOUNDARY,
        context_sect,
    ]
    return "\n\n".join(s for s in sections if s)
