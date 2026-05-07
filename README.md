# 🪡 Loom

> An AI coding assistant that lives in your terminal — powered by any LLM.

[![CI](https://github.com/anasx07/loom/actions/workflows/ci.yml/badge.svg)](https://github.com/anasx07/loom/actions/workflows/ci.yml)
[![Release](https://github.com/anasx07/loom/actions/workflows/release.yml/badge.svg)](https://github.com/anasx07/loom/actions/workflows/release.yml)
[![PyPI](https://img.shields.io/pypi/v/loomcli)](https://pypi.org/project/loomcli/)
[![Python](https://img.shields.io/pypi/pyversions/loomcli)](https://pypi.org/project/loomcli/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

---

## Install

### ⚡ One-liner (no Python required)

**macOS / Linux:**
```sh
curl -fsSL https://raw.githubusercontent.com/anasx07/loom/main/install.sh | sh
```

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/anasx07/loom/main/install.ps1 | iex
```

Downloads a self-contained binary and puts `loom` on your PATH automatically.

---

### 🐍 Via pip / pipx

```sh
pipx install loomcli   # recommended — isolated environment
# or
pip install loomcli
```

Both `loom` and `loomcli` commands are registered after install.

---

### 📦 Manual binary download

Grab the right binary from the [latest release](https://github.com/anasx07/loom/releases/latest):

| Platform            | Binary                      |
|---------------------|-----------------------------|
| Windows x86_64      | `loom-windows-x86_64.exe`   |
| macOS Apple Silicon | `loom-macos-arm64`          |
| macOS Intel         | `loom-macos-x86_64`         |
| Linux x86_64        | `loom-linux-x86_64`         |

Place the binary anywhere on your `PATH` and rename it to `loom`.

---

## Quick start

```sh
loom                            # start interactive session
loom --model gpt-4o             # specific model
loom --provider anthropic       # specific provider
loom --resume                   # resume last session
loom --print "refactor this"    # single-shot, non-interactive
```

On first run, Loom will ask for your API key and save it to `~/.loomcli/config.json`.

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

## What Loom can do

- **Read, write, and edit files** with diff preview and a permission system
- **Run bash commands** with sandboxed output and audit logging
- **Spawn background sub-agents** for long tasks while you keep chatting
- **Context compaction** — auto-summarises history so long sessions never hit limits
- **Skills** — drop Markdown files into `.loomcli/skills/` to give Loom reusable instructions
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
git clone https://github.com/anasx07/loom
cd loom
python -m venv venv && source venv/bin/activate   # Windows: venv\Scripts\activate
pip install -e ".[dev]"
loom
```

Build a standalone binary locally:
```sh
pyinstaller --clean loom.spec
./dist/loom
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
