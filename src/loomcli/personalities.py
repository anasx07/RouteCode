import time
from pathlib import Path
from typing import Optional
from .config import CONFIG_DIR
from .utils import parse_frontmatter


PERSONALITY_DIRS = [
    CONFIG_DIR / "personalities",
    Path(".loomcli") / "personalities",
]


class Personality:
    def __init__(
        self,
        name: str,
        description: str = "",
        prompt: str = "",
        keep_base_instructions: bool = True,
    ):
        self.name = name
        self.description = description
        self.prompt = prompt
        self.keep_base_instructions = keep_base_instructions


BUILTIN_PERSONALITIES = {
    "default": Personality(
        name="default",
        description="Balanced, concise engineering assistant",
        prompt="",
        keep_base_instructions=True,
    ),
    "explanatory": Personality(
        name="explanatory",
        description="Explains implementation choices in detail",
        prompt=(
            "When you make changes, include a #{sha} Insight section "
            "explaining why you chose this approach, what alternatives you considered, "
            "and any trade-offs."
        ),
        keep_base_instructions=True,
    ),
    "concise": Personality(
        name="concise",
        description="Minimal output, just the code",
        prompt=(
            "Be extremely concise. Prefer one-line answers. "
            "Skip explanations unless explicitly asked. "
            "When writing code, omit comments."
        ),
        keep_base_instructions=True,
    ),
}


_personality_cache: dict = None
_personality_cache_mtime: float = 0.0


def load_personalities() -> dict:
    """
    Loads all personalities (builtin and custom).
    Uses MTIME-based caching for better performance.
    """
    global _personality_cache, _personality_cache_mtime

    try:
        current_mtime = 0.0
        for d in PERSONALITY_DIRS:
            if d.exists():
                current_mtime = max(current_mtime, d.stat().st_mtime)
    except Exception:
        current_mtime = time.time()

    if _personality_cache is not None and current_mtime <= _personality_cache_mtime:
        return _personality_cache

    personalities = {}
    for name, p in BUILTIN_PERSONALITIES.items():
        personalities[name] = p

    for base in PERSONALITY_DIRS:
        if base.exists():
            for f in sorted(base.glob("*.md")):
                try:
                    fm, body = parse_frontmatter(f.read_text(encoding="utf-8"))

                    name = fm.get("name", f.stem)
                    personalities[name] = Personality(
                        name=name,
                        description=fm.get("description", ""),
                        prompt=body.strip(),
                        keep_base_instructions=fm.get(
                            "keep-base-instructions", "true"
                        ).lower()
                        == "true",
                    )
                except Exception:
                    pass

    _personality_cache = personalities
    _personality_cache_mtime = current_mtime
    return personalities


def get_active_personality(name: Optional[str] = None) -> Personality:
    personalities = load_personalities()
    if name and name in personalities:
        return personalities[name]
    return personalities.get("default", BUILTIN_PERSONALITIES["default"])


def get_personality_section(name: Optional[str] = None) -> str:
    p = get_active_personality(name)
    if not p.prompt:
        return ""
    return f"## Personality: {p.name}\n{p.prompt}"
