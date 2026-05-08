from typing import List
from rich.table import Table
from ..ui import print_success, print_error
from ..tools import registry
from ..core import LoomContext
from ..domain.attachments import load_attachment


def handle_help(args: List[str], ctx: LoomContext):
    table = Table(
        title="Available Commands", show_header=True, header_style="bold magenta"
    )
    table.add_column("Command", style="command")
    table.add_column("Description")

    from . import get_command_metadata

    metadata = get_command_metadata()
    for cmd, desc in metadata.items():
        table.add_row(cmd, desc)

    ctx.console.print(table)


def handle_tools(args: List[str], ctx: LoomContext):
    table = Table(title="Agent Tools", show_header=True, header_style="bold cyan")
    table.add_column("Tool", style="bold yellow")
    table.add_column("Description")
    for name, desc in registry.list_tools().items():
        table.add_row(name, desc)
    ctx.console.print(table)


def handle_attach(args: List[str], ctx: LoomContext):
    if not args:
        print_error("Usage: /attach <file_path>")
        return
    path = " ".join(args)
    att = load_attachment(path)
    if att is None:
        print_error(f"File not found: {path}")
        return

    if att["type"] == "image":
        msg = {
            "role": "user",
            "content": [
                {"type": "text", "text": f"Attached image: {att['name']}"},
                {
                    "type": "image_url",
                    "image_url": {
                        "url": f"data:{att['mime_type']};base64,{att['data']}"
                    },
                },
            ],
        }
        ctx.state.session_messages.append(msg)
        print_success(f"Attached image: {att['name']} ({att['size'] // 1024}KB)")
    elif att["type"] == "text":
        msg = {
            "role": "user",
            "content": f"[Attached file: {att['name']}]\n```\n{att['content']}\n```",
        }
        ctx.state.session_messages.append(msg)
        line_count = att["content"].count("\n") + 1
        print_success(f"Attached file: {att['name']} ({line_count} lines)")
    else:
        msg = {"role": "user", "content": att.get("content", f"[File: {att['name']}]")}
        ctx.state.session_messages.append(msg)
        print_success(f"Attached: {att['name']}")


def handle_version(args: List[str], ctx: LoomContext):
    from .. import __version__

    ctx.console.print(f"[accent]LoomCLI[/accent] [white]{__version__}[/white]")
    from . import COMMANDS

    ctx.console.print(f"[dim]Python based, {len(COMMANDS)} commands[/dim]")


def handle_clear(args: List[str], ctx: LoomContext):
    ctx.state.session_messages.clear()
    from ..ui import refresh_screen

    refresh_screen(ctx)


async def handle_exit(args: List[str], ctx: LoomContext):
    from .session import handle_save

    await handle_save([], ctx)
    raise EOFError("exit")


def _refresh_screen(ctx: LoomContext):
    """Clears the terminal with the current theme background and repaints the welcome screen."""
    from ..ui import refresh_screen

    refresh_screen(ctx)
