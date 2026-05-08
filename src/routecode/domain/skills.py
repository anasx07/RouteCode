import time
from pathlib import Path
from typing import Any, Dict, List, Optional, TYPE_CHECKING
from ..config import CONFIG_DIR

if TYPE_CHECKING:
    from ..core import RouteCodeContext
from .task_manager import task_manager
from ..utils.helpers import parse_frontmatter


SKILL_DIRS = [
    CONFIG_DIR / "skills",
    Path(".routecode") / "skills",
    Path(__file__).parent / "bundled_skills",
]


class Skill:
    def __init__(self, path: Path):
        self.path = path
        self.name: str = ""
        self.description: str = ""
        self.prompt: str = ""
        self.context: str = "inline"  # inline | fork
        self.model: Optional[str] = None
        self.tools: List[str] = []
        self._parse()

    def _parse(self):
        content = self.path.read_text(encoding="utf-8")
        metadata, body = parse_frontmatter(content)

        if not metadata:
            self.name = self.path.stem
            self.prompt = content
            return

        for key, value in metadata.items():
            if key == "name":
                self.name = value
            elif key == "description":
                self.description = value
            elif key == "context":
                self.context = value if value in ("inline", "fork") else "inline"
            elif key == "model":
                self.model = value if value else None
            elif key == "tools":
                self.tools = [t.strip() for t in value.split(",") if t.strip()]

        if not self.name:
            self.name = self.path.stem
        self.prompt = body.strip()


_skill_cache: Dict[str, Skill] = None
_skill_cache_mtime: float = 0.0


def discover_skills() -> Dict[str, Skill]:
    """
    Discovers all available skills from the configured skill directories.
    Uses MTIME-based caching to avoid expensive filesystem scans.
    """
    global _skill_cache, _skill_cache_mtime

    # Calculate the max MTIME across all existing skill directories
    try:
        current_mtime = 0.0
        for d in SKILL_DIRS:
            if d.exists():
                current_mtime = max(current_mtime, d.stat().st_mtime)
    except Exception:
        # Fallback to re-scanning if stat fails
        current_mtime = time.time()

    if _skill_cache is not None and current_mtime <= _skill_cache_mtime:
        return _skill_cache

    skills = {}
    for skill_dir in SKILL_DIRS:
        if skill_dir.exists():
            for f in sorted(skill_dir.glob("*.md")):
                try:
                    skill = Skill(f)
                    skills[skill.name] = skill
                except Exception:
                    pass

    _skill_cache = skills
    _skill_cache_mtime = current_mtime
    return skills


def get_skill_prompts() -> str:
    skills = discover_skills()
    if not skills:
        return ""
    lines = ["## Available Skills"]
    for name, skill in skills.items():
        context_label = " (runs as sub-agent)" if skill.context == "fork" else ""
        lines.append(f"- {name}: {skill.description}{context_label}")
    return "\n".join(lines)


def run_skill(
    skill: Skill,
    args: str = "",
    ctx: Optional["RouteCodeContext"] = None,
    provider: Optional[Any] = None,
) -> Dict[str, Any]:
    if skill.context == "fork":
        from ..tools.task import _run_sub_agent

        prompt = skill.prompt + "\n\n" + args if args else skill.prompt
        task_id = f"s{abs(hash(prompt)) % 10**7}"
        task_manager.create(skill.name, None, task_id)
        _run_sub_agent(prompt, 10, task_id, ctx, provider=provider)
        record = task_manager.get(task_id)
        if record and record.result:
            return record.result
        return {"success": False, "error": "Skill execution failed"}

    return {
        "success": True,
        "type": "prompt",
        "content": skill.prompt,
        "message": f"Skill '{skill.name}' expanded inline",
    }
