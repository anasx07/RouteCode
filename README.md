# 🪡 RouteCode

> An AI coding assistant that lives in your terminal — powered by any LLM.

[![CI](https://github.com/anasx07/routecode/actions/workflows/ci.yml/badge.svg)](https://github.com/anasx07/routecode/actions/workflows/ci.yml)
[![Release](https://github.com/anasx07/routecode/actions/workflows/release.yml/badge.svg)](https://github.com/anasx07/routecode/actions/workflows/release.yml)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)

---

## Install

### ⚡ One-liner

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

### 🦀 Via Cargo

```sh
cargo install --path apps/cli
```

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
routecode --provider openrouter      # specific provider
routecode --resume last_session      # resume last session
```

On first run, RouteCode will ask for your API key and save it to `~/.routecode/config.json`.

---

## Supported providers

| Provider     | Env var                | Base URL (if custom) |
|--------------|------------------------|----------------------|
| OpenRouter   | `OPENROUTER_API_KEY`   | -                    |
| NVIDIA       | `NVIDIA_API_KEY`       | `https://integrate.api.nvidia.com/v1` |
| OpenCode Zen | `OPENCODE_ZEN_API_KEY` | `https://api.opencode.ai/zen/v1` |
| OpenCode Go  | `OPENCODE_GO_API_KEY`  | `https://api.opencode.ai/go/v1` |
| OpenAI       | `OPENAI_API_KEY`       | -                    |
| Anthropic    | `ANTHROPIC_API_KEY`    | -                    |
| Google       | `GEMINI_API_KEY`       | -                    |
| DeepSeek     | `DEEPSEEK_API_KEY`     | -                    |
| Cloudflare Workers AI | `CLOUDFLARE_API_KEY` (or `account_id:token`) | - |
| Cloudflare AI Gateway | `CLOUDFLARE_API_KEY` (or `account_id:gateway_id:token`) | - |

---

## What RouteCode can do

- **Read and write files** with AI-driven tools
- **Run shell commands** with captured output
- **Session management** — save and resume conversations
- **Token usage tracking** — monitor costs in real-time

---

## Slash commands

| Command    | Description                            |
|------------|----------------------------------------|
| `/model`   | Switch model mid-session               |
| `/resume`  | Resume a saved session                 |
| `/sessions`| List all saved sessions                |
| `/clear`   | Clear the current conversation         |
| `/help`    | Show all commands                      |

---

## Building from source

```sh
git clone https://github.com/anasx07/routecode
cd routecode
cargo build --release -p routecode-cli
# Binary is located at target/release/routecode-cli (or .exe on Windows)
./target/release/routecode-cli
```

---

## Architecture & Project Structure

RouteCode is a Rust workspace consisting of a CLI application and a shared SDK.

```text
apps/cli/               # TUI application
└── src/
    └── ui/             # Ratatui-based interface
libs/sdk/               # Core logic and AI orchestrator
└── src/
    ├── agents/         # AI Provider implementations
    ├── core/           # Orchestrator and message types
    ├── tools/          # AI tools (file_ops, bash, etc.)
    └── utils/          # Storage, costs, and shared utilities
```

---

## Contributing

PRs are welcome. Please open an issue first for significant changes.

```sh
cargo test            # run tests
cargo fmt             # format code
cargo clippy          # lint
```

---

## License

[GPL v3.0](LICENSE)
