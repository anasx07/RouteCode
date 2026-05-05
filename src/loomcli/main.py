import typer
import sys
from typing import Optional
from .repl import LoomREPL
from .ui import console
from .config import config
from .state import state

app = typer.Typer(help="LoomCLI: A Claude Code-like assistant in your terminal.", name="loomcli")


@app.callback(invoke_without_command=True)
def main(
    ctx: typer.Context,
    model: Optional[str] = typer.Option(None, "--model", "-m", help="Model to use"),
    provider: Optional[str] = typer.Option(None, "--provider", help="Provider (openrouter, openai, anthropic, google, deepseek)"),
    resume: Optional[str] = typer.Option(None, "--resume", "-r", help="Resume a saved session by name"),
    print_mode: bool = typer.Option(False, "--print", help="Run a single query and print the result (headless)"),
):
    """LoomCLI: A Claude Code-like assistant in your terminal."""
    if ctx.invoked_subcommand is not None:
        return

    if model:
        config.model = model
    if provider:
        if provider in ("openrouter", "openai", "anthropic", "google", "deepseek"):
            config.provider = provider
        else:
            console.print(f"[error]Unknown provider: {provider}[/error]")
            raise typer.Exit(1)

    repl = LoomREPL()

    if resume:
        from .config import CONFIG_DIR
        session_path = CONFIG_DIR / "sessions" / f"{resume}.json"
        if session_path.exists():
            import json
            data = json.loads(session_path.read_text(encoding="utf-8"))
            state.session_messages = data.get("messages", [])
            state.tokens_used = data.get("tokens_used", 0)
            config.provider = data.get("provider", config.provider)
            config.model = data.get("model", config.model)
            console.print(f"[success]Resumed session: {resume}[/success]")
        else:
            console.print(f"[error]Session not found: {resume}[/error]")
            raise typer.Exit(1)

    if print_mode:
        query = " ".join(sys.argv[sys.argv.index("--") + 1:]) if "--" in sys.argv else ""
        if not query:
            console.print("[error]Usage: loomcli --print -- <query>[/error]")
            raise typer.Exit(1)
        repl.run_single(query)
    else:
        repl.run()


@app.command()
def version():
    """Show version info."""
    console.print("[accent]LoomCLI[/accent] [white]0.1.0[/white]")
    console.print("[dim]Python based, inspired by Claude Code[/dim]")


if __name__ == "__main__":
    app()
