import typer
import sys
from typing import Optional
from .repl import LoomREPL
from . import ui as _ui
from .config import config

app = typer.Typer(help="LoomCLI: An AI assistant for your terminal.", name="loomcli")


@app.callback(invoke_without_command=True)
def main(
    ctx: typer.Context,
    model: Optional[str] = typer.Option(None, "--model", "-m", help="Model to use"),
    provider: Optional[str] = typer.Option(None, "--provider", help="Provider (openrouter, openai, anthropic, google, deepseek)"),
    resume: Optional[str] = typer.Option(None, "--resume", "-r", help="Resume a saved session by name"),
    print_mode: bool = typer.Option(False, "--print", help="Run a single query and print the result (headless)"),
):
    """LoomCLI: An AI assistant for your terminal."""
    if ctx.invoked_subcommand is not None:
        return

    from .core import setup_logging
    setup_logging()

    if model:
        config.model = model
    if provider:
        from .agents.registry import PROVIDER_MAP
        if provider in PROVIDER_MAP:
            config.provider = provider
        else:
            _ui.console.print(f"[error]Unknown provider: {provider}[/error]")
            raise typer.Exit(1)

    repl = LoomREPL()

    if resume:
        from .config import CONFIG_DIR
        session_path = CONFIG_DIR / "sessions" / f"{resume}.json"
        if session_path.exists():
            import json
            data = json.loads(session_path.read_text(encoding="utf-8"))
            repl.ctx.state.session_messages.set_messages(data.get("messages", []))
            repl.state.tokens_used = data.get("tokens_used", 0)
            config.provider = data.get("provider", config.provider)
            config.model = data.get("model", config.model)
            _ui.console.print(f"[success]Resumed session: {resume}[/success]")
        else:
            _ui.console.print(f"[error]Session not found: {resume}[/error]")
            raise typer.Exit(1)

    import asyncio
    if print_mode:
        query = " ".join(sys.argv[sys.argv.index("--") + 1:]) if "--" in sys.argv else ""
        if not query:
            _ui.console.print("[error]Usage: loomcli --print -- <query>[/error]")
            raise typer.Exit(1)
        asyncio.run(repl.run_single(query))
    else:
        asyncio.run(repl.run())


@app.command()
def version():
    """Show version info."""
    _ui.console.print("[accent]LoomCLI[/accent] [white]0.1.0[/white]")
    _ui.console.print("[dim]Python based[/dim]")


if __name__ == "__main__":
    app()
