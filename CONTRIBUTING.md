# Contributing to RouteCode

Thanks for your interest in contributing! Here's how to get started.

## Getting Started

### Prerequisites

- Python 3.10+
- Git

### Setup

```bash
git clone https://github.com/anasx07/routecode.git
cd routecode
python -m venv venv
venv\Scripts\activate  # Windows
# or: source venv/bin/activate  # macOS/Linux
pip install -e ".[dev]"
pre-commit install
```

## Development Workflow

### Running in development mode

```bash
py run_routecode.py
```

For debug logging (opens a separate log window):

```bash
py run_routecode.py --debug
```

### Before committing

Pre-commit hooks run automatically. They check:

- **Ruff** — linting + auto-formatting
- **Trailing whitespace**, missing newlines
- **YAML/TOML/JSON** validity
- **Merge conflict** markers
- **Private key** leaks
- **Debug statements** (`breakpoint()`, `pdb.set_trace()`)

Run manually on all files:

```bash
pre-commit run --all-files
```

### Code style

- Line width: 100 characters
- Quotes: double (`"`)
- Indentation: spaces (4)
- Target: Python 3.10+
- No comments unless essential — code should explain itself
- Use `TracebackType` from `types` or import `traceback` for exception handlers

### Project structure

```
src/routecode/
├── agents/        # AI provider abstraction (LiteLLM, Cloudflare)
├── commands/      # Slash-command system (/help, /model, /theme, etc.)
├── config/        # Configuration, system prompts, model pricing
├── core/          # Core engine (orchestrator, state, events, DI container)
├── domain/        # Business logic (tasks, skills, personalities, git)
├── tools/         # Agent tools (bash, file_edit, glob, grep, etc.)
├── ui/            # Terminal UI
│   ├── dialogs/   # Modal dialogs (theme, model, provider selection)
│   └── repl/      # Main REPL application
└── utils/         # Utilities (storage, logging, costs, errors)
```

### Key architectural patterns

- **AppContainer** (`core/container.py`) — service container with explicit DI wiring and lifecycle phases
- **EventBus** (`core/events.py`) — typed pub/sub for inter-module decoupling
- **ToolRegistry** (`tools/base.py`) — middleware pipeline for tool execution
- **DialogManager** (`ui/dialogs/manager.py`) — centralized Float lifecycle for modal dialogs
- **ToolResult** (`tools/base.py`) — typed results with success/error discrimination

## Pull Requests

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run `pre-commit run --all-files`
5. Submit a PR with a clear description

### PR guidelines

- Keep changes focused — one concern per PR
- Link related issues
- Test your changes locally before submitting
- Follow existing patterns and conventions

## Reporting Issues

Use [GitHub Issues](https://github.com/anasx07/routecode/issues) for:
- Bug reports
- Feature requests
- Questions

Include:
- Your OS and Python version
- Steps to reproduce
- Expected vs actual behavior
- Log output (from `routecode --debug`)

## License

By contributing, you agree that your contributions will be licensed under the GNU GPL v3.0.
