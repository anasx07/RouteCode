from typing import Any, Dict, Optional, TYPE_CHECKING
from pydantic import BaseModel, Field
from .base import BaseTool, registry
from ..skills import discover_skills

if TYPE_CHECKING:
    from ..core import LoomContext


class SkillInput(BaseModel):
    skill: str = Field(..., description="The name of the skill to invoke")
    args: str = Field("", description="Optional arguments to pass to the skill")


class SkillTool(BaseTool):
    name = "skill"
    description = "Invoke a user-defined skill. Skills are reusable workflows defined in .loomcli/skills/ or ~/.loomcli/skills/"
    input_schema = SkillInput

    def prompt(self) -> str:
        skills = discover_skills()
        if not skills:
            return "- skill: Invoke user-defined skills (none currently available)"
        names = ", ".join(skills.keys())
        return f"- skill: Invoke user-defined skills. Available: {names}"

    def execute(self, skill: str, args: str = "", ctx: Optional["LoomContext"] = None, 
                provider: Optional[Any] = None, **kwargs) -> Dict[str, Any]:
        skills = discover_skills()
        if skill not in skills:
            avail = ", ".join(skills.keys()) if skills else "none"
            return {"success": False, "error": f"Skill '{skill}' not found. Available: {avail}"}

        from ..skills import run_skill
        result = run_skill(skills[skill], args, ctx, provider=provider)
        if result.get("type") == "prompt":
            return {
                "success": True,
                "message": f"Skill '{skill}' invoked. Its prompt has been added to context."
            }
        return result
