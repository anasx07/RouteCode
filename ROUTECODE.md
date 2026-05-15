# RouteCode Project Instructions
- Always use Rust for core logic and CLI enhancements.
- Be concise in your responses.
- If you create a file, always explain why.

## Recent Enhancements
- **Persistent History**: The REPL saves your command history in `.routecode/sessions`.
- **Comprehensive Project Context**: The AI now automatically reads `ROUTECODE.md`, `README.md`, and `Cargo.toml` to understand your project better.
- **Enhanced Tools**: 
    - `file_ops` supports reading and writing files.
    - `bash` tool executes terminal commands.
- **Improved UI**: Added a command menu and status bar with token usage.
- **Testing Suite**: Use `cargo test` to run unit tests.

## Development & Testing
- To run tests: `cargo test`
- Sessions are stored in `.routecode/sessions`
- Configuration is in `~/.routecode/config.json`
