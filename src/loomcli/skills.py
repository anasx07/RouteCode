import os
import re
from pathlib import Path
from typing import Any, Dict, List, Optional
from .config import CONFIG_DIR
from .task_manager import task_manager


SKILL_DIRS = [
    CONFIG_DIR / "skills",
    Path(".loomcli") / "skills",
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
        # Parse YAML frontmatter (between --- delimiters)
        fm_match = re.match(r'^---\s*\n(.*?)\n---\s*\n?(.*)', content, re.DOTALL)
        if not fm_match:
            self.name = self.path.stem
            self.description = ""
            self.prompt = content
            return

        frontmatter = fm_match.group(1)
        body = fm_match.group(2).strip()

        for line in frontmatter.split("\n"):
            line = line.strip()
            if ":" in line:
                key, _, value = line.partition(":")
                key = key.strip().lower()
                value = value.strip().strip('"').strip("'")
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
        self.prompt = body


def discover_skills() -> Dict[str, Skill]:
    skills = {}
    for skill_dir in SKILL_DIRS:
        if skill_dir.exists():
            for f in sorted(skill_dir.glob("*.md")):
                try:
                    skill = Skill(f)
                    skills[skill.name] = skill
                except Exception:
                    pass
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


def run_skill(skill: Skill, args: str = "") -> Dict[str, Any]:
    if skill.context == "fork":
        from .tools.task import _run_sub_agent
        prompt = skill.prompt + "\n\n" + args if args else skill.prompt
        task_id = f"s{abs(hash(prompt)) % 10**7}"
        task_manager.create(skill.name, None, task_id)
        _run_sub_agent(prompt, 10, task_id)
        record = task_manager.get(task_id)
        if record and record.result:
            return record.result
        return {"success": False, "error": "Skill execution failed"}

    return {
        "success": True,
        "type": "prompt",
        "content": skill.prompt,
        "message": f"Skill '{skill.name}' expanded inline"
    }
