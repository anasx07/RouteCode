import subprocess
from concurrent.futures import ThreadPoolExecutor
from typing import Dict, Optional

def get_git_context() -> str:
    """
    Gathers Git status, logs, and branch info in parallel.
    Returns a formatted string suitable for a system prompt section.
    """
    def run_git(cmd: str) -> str:
        try:
            return subprocess.run(
                cmd, 
                shell=True, 
                capture_output=True, 
                text=True, 
                timeout=3
            ).stdout.strip()
        except Exception:
            return ""

    with ThreadPoolExecutor(max_workers=3) as executor:
        f_status = executor.submit(run_git, "git status --short")
        f_log = executor.submit(run_git, "git log --oneline -5")
        f_branch = executor.submit(run_git, "git rev-parse --abbrev-ref HEAD")
        
        status = f_status.result()
        log = f_log.result()
        branch = f_branch.result()

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
