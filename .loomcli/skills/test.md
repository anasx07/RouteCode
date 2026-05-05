---
name: test
description: Run pytest with verbose output
context: fork
tools: bash, glob
---
Run `pytest -v` with the following configuration:
- Use `-x` to stop on first failure
- Show stdout with `-s`
- Target the `tests/` directory
- If tests fail, read the failing test files and try to fix them
