import os
import sys
import asyncio
from typing import Optional, TYPE_CHECKING
from ..tools import registry

if TYPE_CHECKING:
    from ..core import RouteCodeContext


SYSTEM_PROMPT_DYNAMIC_BOUNDARY = "__DYNAMIC__"


def _build_identity_section() -> str:
    return """You are RouteCode CLI, a senior software engineer and collaborative peer programmer.
You have access to local file system tools and a shell environment.
When responding, start with a <thought> block for your internal reasoning and plan, then provide your response."""


def _build_workspace_section() -> str:
    import platform
    from datetime import datetime

    cwd = os.getcwd()
    os_name = platform.system()
    os_platform = sys.platform
    date_str = datetime.now().strftime("%A, %B %d, %Y")
    tmp_dir = os.path.expanduser("~/.routecode/tmp")

    project_structure = ""
    try:
        import subprocess

        # Try to get git tree first
        res = subprocess.run(
            ["git", "ls-tree", "-r", "--name-only", "HEAD"],
            capture_output=True,
            text=True,
            check=False,
            shell=True if os_name == "Windows" else False,
        )
        if res.returncode == 0:
            lines = res.stdout.strip().splitlines()
            if len(lines) > 60:
                project_structure = (
                    "\n".join(lines[:60]) + f"\n... ({len(lines) - 60} more files)"
                )
            else:
                project_structure = "\n".join(lines)
        else:
            # Fallback to listing current directory
            files = os.listdir(cwd)
            project_structure = "\n".join(files[:40])
            if len(files) > 40:
                project_structure += f"\n... ({len(files) - 40} more items)"
    except Exception:
        project_structure = "Unable to retrieve directory structure."

    return f"""## Context & Environment
- **OS**: {os_platform} ({os_name})
- **Date**: {date_str}
- **Workspace**: `{cwd}`
- **Temporary Directory**: `{tmp_dir}`
- **Project Structure**:
```
{project_structure}
```
"""


def _build_tools_section() -> str:
    tool_prompts = []
    for name, tool in registry.list_tools_with_prompts().items():
        tool_prompts.append(tool)

    tools_header = """## Available Tools & Sub-Agents
### Capabilities
- **File Manipulation**: `read_file`, `write_file`, `replace` — for surgical and bulk changes.
- **Searching**: `glob`, `grep_search` — for codebase exploration and architectural mapping.
- **System Access**: `run_shell_command` — for running tests, linting, and environment interaction.
- **Web Research**: `web_fetch`, `google_web_search` — for up-to-date documentation and troubleshooting.

### Sub-Agents (via `task`)
Use the `task` tool to spawn specialized sub-agents for complex work:
- **codebase_investigator**: For deep analysis, dependency mapping, and architectural reviews.
- **cli_help**: For internal questions about RouteCode features, schemas, and configurations.
- **generalist**: For high-volume, repetitive batch tasks or boilerplate generation.

### Skills
- **skill-creator**: Dynamically extend my capabilities by creating new reusable scripts.
- **find-skills**: Discover and list all currently installed skills in the workspace.

---
"""
    if tool_prompts:
        return tools_header + "### Detailed Tool Reference\n" + "\n".join(tool_prompts)
    return tools_header


def _build_behavior_section() -> str:
    return """## Behavior
- **Development Lifecycle**: Always follow a **Research -> Strategy -> Execution** cycle:
    1. **Research**: Explore the codebase, read relevant files, and understand the context before proposing changes.
    2. **Strategy**: Formulate a clear plan in your <thought> block, identifying the specific files and logic that need modification.
    3. **Execution**: Apply the changes precisely using the available tools, following the plan you established.
- **Engineering Standards**: Prioritize idiomatic, type-safe, and maintainable code. Verify all changes with tests and workspace-specific linting/type-checking (e.g., `pytest`, `ruff`, `mypy`) using the `run_shell_command` tool when applicable.
- Be concise. Use tools when needed, not for every response.
- **CRITICAL: When creating or modifying code, ALWAYS use `write_file` or `replace` tools to apply the changes directly to the filesystem. Avoid sending raw code blocks in chat unless the user specifically asks for a snippet without applying it.**
- Prefer `read_file` over `run_shell_command cat` for reading files. read_file returns results in cat -n format with line numbers.
- Use `glob` and `grep_search` for codebase exploration before editing.
- When editing, use `replace` for surgical changes, `write_file` for new files.
- Use `task` for complex multi-step work that requires autonomous execution.
- ALWAYS use read_file to view files before editing them.

## Workflow Rules
- **Validation**: A task is only "complete" when it has been empirically verified via tests or reproduction scripts.
- **Proactiveness**: Persist through errors and diagnose failures autonomously unless a significant architectural pivot is required.
- **Brevity**: Aim for concise, high-signal technical communication. Avoid filler or conversational fluff.

## Parallel Execution
The following tools are concurrency-safe and can be run in parallel:
- `read_file`, `glob`, `grep_search`, `web_fetch` — read-only and safe to batch.

Tools like `run_shell_command`, `replace`, `write_file` run serially (one at a time).
When you have multiple read operations, batch them together in one response."""


def _build_memory_section(ctx: "RouteCodeContext") -> str:
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


async def _build_configuration_tiers_async(ctx: "RouteCodeContext") -> str:
    """Build the tiered configuration section (Global, Project, Sub, Private)."""
    cwd = os.getcwd()
    project_hash = str(abs(hash(cwd)) % 10**8)

    tiers = []

    # 1. Project Instructions (./ROUTECODE.md)
    project_instr = await _get_file_content_cached(os.path.join(cwd, "ROUTECODE.md"))
    if project_instr:
        tiers.append(f"### 1. Project Instructions (Shared)\n{project_instr}")
    else:
        tiers.append("### 1. Project Instructions (Shared)\n*None detected in root.*")

    # 2. Subdirectory Instructions (**/ROUTECODE.md)
    sub_instr = []
    try:
        import glob

        # Only look 2 levels deep to avoid performance issues
        for f in glob.glob("**/ROUTECODE.md", recursive=True):
            if os.path.abspath(f) == os.path.abspath(os.path.join(cwd, "ROUTECODE.md")):
                continue
            content = await _get_file_content_cached(f)
            if content:
                sub_instr.append(f"#### {f}\n{content}")
    except Exception:
        pass

    if sub_instr:
        tiers.append(
            "### 2. Subdirectory Instructions (Module-Scoped)\n" + "\n".join(sub_instr)
        )
    else:
        tiers.append(
            "### 2. Subdirectory Instructions (Module-Scoped)\n*None detected.*"
        )

    # 3. Private Project Memory (~/.routecode/tmp/<id>/memory/MEMORY.md)
    private_mem_path = os.path.expanduser(
        f"~/.routecode/tmp/{project_hash}/memory/MEMORY.md"
    )
    private_mem = await _get_file_content_cached(private_mem_path)
    if private_mem:
        tiers.append(f"### 3. Private Project Memory (Local-Only)\n{private_mem}")
    else:
        tiers.append(
            f"### 3. Private Project Memory (Local-Only)\n*No private memory found at {private_mem_path}*"
        )

    # 4. Global Personal Memory (~/.routecode/ROUTECODE.md)
    global_mem_path = os.path.expanduser("~/.routecode/ROUTECODE.md")
    global_mem = await _get_file_content_cached(global_mem_path)
    if global_mem:
        tiers.append(f"### 4. Global Personal Memory (Preferences)\n{global_mem}")
    else:
        tiers.append(
            "### 4. Global Personal Memory (Preferences)\n*No global personal memory found.*"
        )

    return "## Configuration Tiers\n" + "\n\n".join(tiers)


async def _build_routecode_section_async() -> Optional[str]:
    # This is replaced by the configuration tiers logic
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
    from ..domain.git import get_git_context_async

    return await get_git_context_async()


def _build_skill_section() -> str:
    from ..domain.skills import get_skill_prompts

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


async def compute_system_prompt(ctx: "RouteCodeContext") -> str:
    from ..domain.personalities import get_personality_section, get_active_personality

    if ctx.config.personality:
        from ..domain.personalities import load_personalities

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
        _build_configuration_tiers_async(ctx),
        _build_git_section_async(),
        _build_context_section_async(),
    )

    config_tiers_sect, git_sect, context_sect = dynamic_results

    sections += [
        _build_workspace_section(),
        _build_tools_section(),
        _build_skill_section(),
        _build_env_section(),
        config_tiers_sect,
        _build_memory_section(ctx),
        git_sect,
        get_personality_section(),
        SYSTEM_PROMPT_DYNAMIC_BOUNDARY,
        context_sect,
    ]
    return "\n\n".join(s for s in sections if s)
