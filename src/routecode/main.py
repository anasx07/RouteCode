import typer
import sys
from typing import Optional
from .ui.repl import RouteCodeREPL
from . import ui as _ui
from .config import config

app = typer.Typer(
    help="routecode: An AI assistant for your terminal.", name="routecode"
)


@app.callback(invoke_without_command=True)
def main(
    ctx: typer.Context,
    model: Optional[str] = typer.Option(None, "--model", "-m", help="Model to use"),
    provider: Optional[str] = typer.Option(
        None,
        "--provider",
        help="Provider (openrouter, openai, anthropic, google, deepseek)",
    ),
    resume: Optional[str] = typer.Option(
        None, "--resume", "-r", help="Resume a saved session by name"
    ),
    print_mode: bool = typer.Option(
        False, "--print", help="Run a single query and print the result (headless)"
    ),
    update: bool = typer.Option(
        False, "--update", help="Check for and install the latest version of RouteCode"
    ),
):
    """routecode: An AI assistant for your terminal."""
    if ctx.invoked_subcommand is not None:
        return

    from .utils.logger import setup_logging

    setup_logging()

    if update:
        from .updater import check_for_update, perform_update

        _ui.console.print("[accent]Checking for updates...[/accent]")
        info = check_for_update()
        if info.error:
            _ui.console.print(f"[dim]{info.error}[/dim]")
        elif info.is_available:
            _ui.console.print(
                f"[accent]Update available:[/accent] "
                f"[white]{info.current_version}[/white] → [white]{info.latest_version}[/white]"
            )
            perform_update(info, console=_ui.console)
            raise typer.Exit()
        else:
            _ui.console.print(
                f"[success]Already up to date[/success] [dim]({info.current_version})[/dim]"
            )
            raise typer.Exit()

    if model:
        config.model = model
    if provider:
        from .agents.registry import PROVIDER_MAP

        if provider in PROVIDER_MAP:
            config.provider = provider
        else:
            _ui.console.print(f"[error]Unknown provider: {provider}[/error]")
            raise typer.Exit(1)

    repl = RouteCodeREPL()

    if resume:
        from .core import load_session

        state = load_session(resume)
        if state:
            repl.ctx.state = state
            # Synchronize config with restored state if present
            if state.provider:
                config.provider = state.provider
            if state.model:
                config.model = state.model
            _ui.console.print(f"[success]Resumed session: {resume}[/success]")
        else:
            _ui.console.print(
                f"[error]Session not found or corrupted: {resume}[/error]"
            )
            raise typer.Exit(1)

    import asyncio

    if print_mode:
        query = (
            " ".join(sys.argv[sys.argv.index("--") + 1 :]) if "--" in sys.argv else ""
        )
        if not query:
            _ui.console.print("[error]Usage: routecode --print -- <query>[/error]")
            raise typer.Exit(1)
        asyncio.run(repl.run_single(query))
    else:
        asyncio.run(repl.run())


@app.command()
def version():
    """Show version info."""
    from . import __version__

    _ui.console.print(f"[accent]routecode[/accent] [white]{__version__}[/white]")
    _ui.console.print("[dim]Python based[/dim]")


if __name__ == "__main__":
    app()
