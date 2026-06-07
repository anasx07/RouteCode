# 🪡 RouteCode

> An AI coding assistant that lives in your terminal — powered by any LLM.

[![CI](https://github.com/anasx07/routecode/actions/workflows/ci.yml/badge.svg)](https://github.com/anasx07/routecode/actions/workflows/ci.yml)
[![Release](https://github.com/anasx07/routecode/actions/workflows/release.yml/badge.svg)](https://github.com/anasx07/routecode/actions/workflows/release.yml)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
[![Discord](https://img.shields.io/badge/Discord-Join%20Community-5865F2?logo=discord&logoColor=white)](https://discord.gg/RFVNKQUXY2)

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

## Configuration

All persistent settings live in `~/.routecode/config.json` (created on first run).
You can edit the file by hand — unknown fields are ignored, and missing fields
fall back to their defaults. The most useful fields:

| Field                   | Type      | Default        | Description |
|-------------------------|-----------|----------------|-------------|
| `model`                 | `string`  | `gpt-4o`       | Default model used at startup. |
| `provider`              | `string`  | `openai`       | Default provider (see [Supported providers](#supported-providers)). |
| `api_keys`              | `object`  | `{}`           | Map of provider id → API key. |
| `thinking_level`        | `string`  | `default`      | One of `default` / `low` / `medium` / `high` (provider-dependent). |
| `sub_agents_enabled`    | `bool`    | `true`         | Allow the orchestrator to spawn sub-agents for subtasks. |
| `retry_policy`          | `object`  | `{"strategy": "disabled"}` | Retry strategy for failed provider requests. Tagged shape: `{"strategy": "disabled"}`, `{"strategy": "qir"}`, or `{"strategy": "exponential_backoff", "max_attempts": 5, "base_secs": 1.0, "jitter": true}`. Currently only `qir` and `disabled` are honored by the orchestrator; `exponential_backoff` is reserved. See below. |
| `vertex_project`        | `string`  | `""`           | GCP project id, required when `provider = "vertex"`. |
| `vertex_location`       | `string`  | `us-central1`  | GCP region for Vertex. |
| `allowlist`             | `string[]`| `[]`           | Extra filesystem paths the bash / file tools are allowed to touch. |

#### `retry_policy` strategies

| `strategy`              | Behavior |
|-------------------------|----------|
| `disabled`              | No retry on failure. Default. |
| `qir`                   | **Experimental.** Quick Infinite Retry — re-send failed requests immediately with no delay and no attempt limit. **Use at your own risk** — aggressive retrying can rate-limit, suspend, or permanently ban your account with Anthropic, OpenAI, Google Gemini, OpenRouter, Cloudflare, Vertex, OpenCode Zen, etc. The RouteCode project and its authors accept no responsibility for bans, lost credits, or other consequences. Toggle this in **Settings → Engine & Models → Quick Infinite Retry**. |
| `exponential_backoff`   | Reserved for future use. The fields `max_attempts`, `base_secs`, and `jitter` are accepted but not yet honored at the call site. |

> **Note:** `retry_policy` is snapshotted at the start of each request and is
> NOT re-read mid-run. If you want to stop a runaway retry loop, use the Stop
> button (sends a cancel signal) — don't just toggle QIR off.

> **Migration:** Old configs with `quick_infinite_retry: true` (or
> `retry_policy: true`) are auto-migrated to `retry_policy: {"strategy": "qir"}`
> on load. A deprecation warning is logged. The legacy field is never written
> back out — save once to clean up.

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

## Community

Join the RouteCode Discord — where bugs get caught, features get shaped, and contributors find each other.

[![Discord](https://img.shields.io/badge/Discord-Join%20Community-5865F2?logo=discord&logoColor=white)](https://discord.gg/RFVNKQUXY2)

- 💬 General chat and questions
- 🐛 Bug reports and help
- 💡 Feature requests and roadmap discussion
- 🎉 Show and tell — share what you built

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
