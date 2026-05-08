---
name: lint
description: Run ruff linter and auto-fix issues
context: fork
tools: bash, file_edit, glob
---
Run `ruff check --fix src/` to lint and auto-fix the codebase.
If ruff is not available, suggest installing it with `pip install ruff`.
Show the list of fixed issues.
