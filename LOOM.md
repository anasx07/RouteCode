# Loom Project Instructions
- Always use Python for scripts.
- Be concise in your responses.
- If you create a file, always explain why.

## Recent Enhancements
- **Persistent History**: The REPL now saves your command history in `~/.loomcli/history`.
- **Comprehensive Project Context**: The AI now automatically reads `LOOM.md`, `README.md`, and `pyproject.toml` to understand your project better.
- **Enhanced Tools**: 
    - `file_edit` now supports an `allow_multiple` flag for bulk replacements.
    - `bash` tool now reports the current working directory.
- **Improved UI**: Added a spinner and elapsed time tracking to the thinking indicator.
- **Testing Suite**: A new `tests/` directory contains unit tests for core functionality. Run them with `pytest`.

## Development & Testing
- To run tests: `$env:PYTHONPATH="src"; python -m pytest`
- History is stored in `~/.loomcli/history`
- Configuration is in `~/.loomcli/config.json`
