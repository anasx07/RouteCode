import asyncio
import shlex
from typing import Dict, Optional

async def get_git_context_async() -> str:
    """
    Gathers Git status, logs, and branch info in parallel asynchronously.
    Returns a formatted string suitable for a system prompt section.
    """
    async def run_git_async(cmd: str) -> str:
        try:
            proc = await asyncio.create_subprocess_shell(
                cmd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE
            )
            stdout, stderr = await asyncio.wait_for(proc.communicate(), timeout=3.0)
            return stdout.decode().strip()
        except Exception:
            return ""

    results = await asyncio.gather(
        run_git_async("git status --short"),
        run_git_async("git log --oneline -5"),
        run_git_async("git rev-parse --abbrev-ref HEAD")
    )
    
    status, log, branch = results

    parts = []
    if branch:
        parts.append(f"Current branch: {branch}")
    if status:
        lines = status.split("\n")[:20]
        parts.append(f"Changed files ({len(lines)}):\n" + "\n".join(lines))
    if log:
        parts.append(f"Recent commits:\n{log[:500]}")
    
    if not parts:
        return ""
        
    return "## Git Context\n" + "\n".join(parts)

def get_git_context() -> str:
    """Synchronous fallback that runs the async version."""
    try:
        loop = asyncio.get_running_loop()
        if loop.is_running():
            # If we're in a loop, we can't easily run it sync without a thread
            from concurrent.futures import ThreadPoolExecutor
            with ThreadPoolExecutor() as executor:
                return executor.submit(asyncio.run, get_git_context_async()).result()
    except RuntimeError:
        return asyncio.run(get_git_context_async())
    
    # Fallback if loop detection fails or other issues
    return ""
