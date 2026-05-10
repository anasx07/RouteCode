from typing import Any, Dict, Optional, TYPE_CHECKING
from pathlib import Path
from pydantic import BaseModel, Field
from .base import BaseTool
from ..domain.skills import discover_skills

if TYPE_CHECKING:
    from ..core import RouteCodeContext


class SkillInput(BaseModel):
    skill: str = Field(..., description="The name of the skill to invoke")
    args: str = Field("", description="Optional arguments to pass to the skill")


class SkillTool(BaseTool):
    name = "skill"
    description = "Invoke a user-defined skill. Skills are reusable workflows defined in .routecode/skills/ or ~/.routecode/skills/"
    input_schema = SkillInput

    def prompt(self) -> str:
        skills = discover_skills()
        if not skills:
            return "- skill: Invoke user-defined skills (none currently available)"
        names = ", ".join(skills.keys())
        return f"- skill: Invoke user-defined skills. Available: {names}"

    def _run(
        self,
        skill: str,
        args: str = "",
        ctx: Optional["RouteCodeContext"] = None,
        provider: Optional[Any] = None,
        **kwargs,
    ) -> Dict[str, Any]:
        skills = discover_skills()
        if skill not in skills:
            avail = ", ".join(skills.keys()) if skills else "none"
            return {
                "success": False,
                "error": f"Skill '{skill}' not found. Available: {avail}",
            }

        from ..domain.skills import run_skill

        return run_skill(skills[skill], args, ctx, provider=provider)


class SkillCreatorInput(BaseModel):
    name: str = Field(..., description="Name of the skill (e.g., 'deploy-lambda')")
    description: str = Field(
        ..., description="Short description of what the skill does"
    )
    prompt: str = Field(
        ..., description="The system prompt or instructions for the skill"
    )
    context: str = Field(
        "inline",
        description="Execution context: 'inline' (appends prompt) or 'fork' (runs as sub-agent)",
    )


class SkillCreatorTool(BaseTool):
    name = "skill_creator"
    description = "Create a new reusable skill. This extends your capabilities with a custom workflow."
    input_schema = SkillCreatorInput

    def prompt(self) -> str:
        return "- skill_creator: Create a new reusable skill to extend your own capabilities."

    def _run(
        self,
        name: str,
        description: str,
        prompt: str,
        context: str = "inline",
        **kwargs,
    ) -> Dict[str, Any]:
        # Normalize name for filename
        safe_name = name.lower().replace(" ", "-").strip()

        # We prefer local project skills dir
        skill_dir = Path(".routecode") / "skills" / safe_name
        skill_dir.mkdir(parents=True, exist_ok=True)

        file_path = skill_dir / "README.md"

        content = f"""---
name: {name}
description: {description}
context: {context}
---

{prompt}
"""
        try:
            file_path.write_text(content, encoding="utf-8")
            return {
                "success": True,
                "message": f"Skill '{name}' created successfully at {skill_dir}",
                "path": str(file_path),
            }
        except Exception as e:
            return {"success": False, "error": f"Failed to create skill: {str(e)}"}


class EmptySchema(BaseModel):
    pass


class FindSkillsTool(BaseTool):
    name = "find_skills"
    description = "List all currently installed skills and their descriptions."
    input_schema = EmptySchema

    def prompt(self) -> str:
        return "- find_skills: List all installed skills and discover your extended capabilities."

    def _run(self, **kwargs) -> Dict[str, Any]:
        skills = discover_skills()
        if not skills:
            return {
                "success": True,
                "skills": [],
                "message": "No skills installed yet.",
            }

        result = []
        for s in skills.values():
            result.append(
                {"name": s.name, "description": s.description, "context": s.context}
            )

        return {"success": True, "skills": result}
