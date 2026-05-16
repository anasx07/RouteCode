# Contributing to RouteCode

Thanks for your interest in contributing! RouteCode is a Rust-based AI coding assistant.

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable)
- Git

### Setup

```bash
git clone https://github.com/anasx07/routecode.git
cd routecode
cargo build
```

## Development Workflow

### Running in development mode

```bash
cargo run -p routecode-cli
```

For debug logging:

```bash
cargo run -p routecode-cli -- --debug
```

### Before committing

Please ensure your code passes the following checks:

```bash
cargo fmt --all -- --check    # Check formatting
cargo clippy --workspace      # Run linter
cargo test --workspace        # Run tests
```

### Project Structure

RouteCode is a Rust workspace:

- `apps/cli/`: The TUI application (Ratatui-based).
- `libs/sdk/`: Core logic, AI provider implementations, and tools.
  - `src/agents/`: AI Provider implementations (OpenAI, Anthropic, Gemini, etc.).
  - `src/core/`: Orchestrator, message types, and configuration.
  - `src/tools/`: AI tools (bash, file_ops, navigation).
  - `src/utils/`: Storage, costs, and token counting.

## Pull Requests

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run `cargo test --workspace`
5. Submit a PR with a clear description

### PR guidelines

- Keep changes focused — one concern per PR.
- Link related issues.
- Follow existing patterns and conventions.
- Add tests for new features or bug fixes.

## Reporting Issues

Use [GitHub Issues](https://github.com/anasx07/routecode/issues) for bug reports and feature requests.

## License

By contributing, you agree that your contributions will be licensed under the GNU GPL v3.0.
