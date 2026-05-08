# 🪡 RouteCode

> An AI coding assistant that lives in your terminal — powered by any LLM.

[![CI](https://github.com/anasx07/routecode/actions/workflows/ci.yml/badge.svg)](https://github.com/anasx07/routecode/actions/workflows/ci.yml)
[![Release](https://github.com/anasx07/routecode/actions/workflows/release.yml/badge.svg)](https://github.com/anasx07/routecode/actions/workflows/release.yml)
[![PyPI](https://img.shields.io/pypi/v/routecode)](https://pypi.org/project/routecode/)
[![Python](https://img.shields.io/pypi/pyversions/routecode)](https://pypi.org/project/routecode/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

---

## Install

### ⚡ One-liner (no Python required)

**macOS / Linux:**
```sh
curl -fsSL https://raw.githubusercontent.com/anasx07/routecode/main/install.sh | sh
```

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/anasx07/routecode/main/install.ps1 | iex
```

Downloads a self-contained binary and puts `routecode` on your PATH automatically.

---

### 🐍 Via pip / pipx

```sh
pipx install routecode   # recommended — isolated environment
# or
pip install routecode
```

Both `routecode` commands are registered after install.

---

### 📦 Manual binary download

Grab the right binary from the [latest release](https://github.com/anasx07/routecode/releases/latest):

| Platform            | Binary                      |
|---------------------|-----------------------------|
| Windows x86_64      | `routecode-windows-x86_64.exe`   |
| macOS Apple Silicon | `routecode-macos-arm64`          |
| macOS Intel         | `routecode-macos-x86_64`         |
| Linux x86_64        | `routecode-linux-x86_64`         |

Place the binary anywhere on your `PATH` and rename it to `routecode`.

---

## Quick start

```sh
routecode                            # start interactive session
routecode --model gpt-4o             # specific model
routecode --provider anthropic       # specific provider
routecode --resume                   # resume last session
routecode --print "refactor this"    # single-shot, non-interactive
```

On first run, RouteCode will ask for your API key and save it to `~/.routecode/config.json`.

---

## Supported providers

| Provider     | Env var                |
|--------------|------------------------|
| Anthropic    | `ANTHROPIC_API_KEY`    |
| OpenAI       | `OPENAI_API_KEY`       |
| Google       | `GEMINI_API_KEY`       |
| DeepSeek     | `DEEPSEEK_API_KEY`     |
| OpenRouter   | `OPENROUTER_API_KEY`   |

---

## What RouteCode can do

- **Read, write, and edit files** with diff preview and a permission system
- **Run bash commands** with captured output and audit logging
- **Spawn background sub-agents** for long tasks while you keep chatting
- **Context compaction** — auto-summarises history so long sessions never hit limits
- **Skills** — drop Markdown files into `.routecode/skills/` to give RouteCode reusable instructions
- **Session resume** — every conversation is saved; pick up where you left off
- **Themes & personalities** — customise the look and the agent's tone

---

## Slash commands

| Command    | Description                            |
|------------|----------------------------------------|
| `/config`  | View and set provider, model, API keys |
| `/model`   | Switch model mid-session               |
| `/session` | Save, load, list, or clear sessions    |
| `/memory`  | Manage persistent memory               |
| `/tasks`   | View and manage background tasks       |
| `/theme`   | Switch UI theme                        |
| `/clear`   | Clear the current conversation         |
| `/help`    | Show all commands                      |

---

## Building from source

```sh
git clone https://github.com/anasx07/routecode
cd routecode
python -m venv venv && source venv/bin/activate   # Windows: venv\Scripts\activate
pip install -e ".[dev]"
routecode
```

Build a standalone binary locally:
```sh
pyinstaller --clean routecode.spec
./dist/routecode
```

---

## Architecture & Project Structure

RouteCode follows a domain-driven, modular architecture designed for stability and scalability.

```text
src/routecodecli/
├── agents/             # Provider-specific implementations (LiteLLM, Anthropic, etc.)
├── commands/           # CLI slash-command handlers (/config, /session, etc.)
├── config/             # Global settings, models database, and system prompt logic
├── core/               # Orchestration, event bus, context management, and state
├── domain/             # Business logic (Task management, Skills, Git, Personalities)
├── tools/              # AI-accessible tools (file_edit, bash, webfetch, etc.)
├── ui/                 # TUI components, theme engine, and dialogs
│   └── repl/           # Core interactive application logic (split-pane TUI)
└── utils/              # Shared utilities (logging, atomic storage, cost estimation)
```

---

## Contributing

PRs are welcome. Please open an issue first for significant changes.

```sh
pip install -e ".[dev]"
pytest            # run tests
ruff check src/   # lint
```

---

## License

[MIT](LICENSE)
