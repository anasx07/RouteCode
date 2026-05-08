"""
routecode._release — importable entry point for the `routecode-release` command.
The actual implementation lives in scripts/release.py at the repo root.
This thin shim re-exports `main` so pyproject.toml [project.scripts] can
find it after `pip install -e .`
"""

import sys
from pathlib import Path


# When installed as a package, run the release script directly
def main():
    # Find scripts/release.py relative to this file's package root
    pkg_root = Path(__file__).resolve().parent  # src/routecode/
    repo_root = pkg_root.parent.parent  # repo root
    script = repo_root / "scripts" / "release.py"

    if not script.exists():
        # Fallback: inline the minimal bootstrap if not in a dev install
        print("release.py not found at", script)
        print("Run from the repo root: python scripts/release.py")
        sys.exit(1)

    # Execute the script in its own namespace so __file__ resolves correctly
    import runpy

    runpy.run_path(str(script), run_name="__main__")


if __name__ == "__main__":
    main()
