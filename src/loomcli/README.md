# 📦 loomcli

Internal package for the Loom AI assistant.

## 🏛 Architecture

LoomCLI is built on a **Modular, Event-Driven Architecture**. The codebase is strictly partitioned into functional domains to ensure high maintainability and prevent circular dependencies.

### Package Responsibilities

| Package | Responsibility |
|:---|:---|
| `agents` | LLM provider abstractions and API transport logic. |
| `commands` | Interactive slash-command implementations (`/config`, `/session`). |
| `config` | Global state, models database, and dynamic system prompt generation. |
| `core` | The central engine: Orchestration, Event Bus, Context, and State. |
| `domain` | High-level business logic: Task Manager, Skills system, and Git integration. |
| `tools` | Discrete capabilities exposed to the LLM (file I/O, shell, web). |
| `ui` | Rendering layer: Theme engine, Dialogs, and Terminal management. |
| `ui/repl` | The main interactive application and TUI event loop. |
| `utils` | Low-level cross-cutting concerns: storage, logging, and metrics. |

## 🛠 Internal Design Patterns

1. **Context-Aware Orchestration**: All tools and commands receive a `LoomContext` which injects necessary state and services without tight coupling.
2. **Event-Driven UI**: The UI reacts to core events (e.g., `task.completed`, `session.turn_complete`) via a central `bus`.
3. **Atomic Persistence**: Configuration and session data are managed via `AtomicJsonStore` to prevent data corruption during crashes.
4. **Tool Permission System**: Destructive tools (like `bash` or `file_edit`) are gated by a user-approval loop and a configurable `allowlist`.

## 🚀 Execution Flow

1. `main.py` (CLI entry) initializes the `config` and `LoomREPL`.
2. `ui/repl/app.py` starts the `prompt_toolkit` event loop.
3. User input is dispatched to either `commands` (if prefixed with `/`) or the `core.orchestrator`.
4. The `AgentOrchestrator` manages the LLM conversation loop, invoking `tools` as requested by the model.
5. All output is streamed in real-time to the TUI via `ui/repl/handlers.py`.

---
*Loom Architecture v2.0 - Stabilized and Modularized.*
