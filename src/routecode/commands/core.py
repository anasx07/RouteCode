from typing import List
from rich.table import Table
from ..ui import print_success, print_error
from ..tools import registry
from ..core import RouteCodeContext
from ..domain.attachments import load_attachment


def handle_help(args: List[str], ctx: RouteCodeContext):
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


def handle_tools(args: List[str], ctx: RouteCodeContext):
    table = Table(title="Agent Tools", show_header=True, header_style="bold cyan")
    table.add_column("Tool", style="bold yellow")
    table.add_column("Description")
    for name, desc in registry.list_tools().items():
        table.add_row(name, desc)
    ctx.console.print(table)


def handle_attach(args: List[str], ctx: RouteCodeContext):
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


def handle_version(args: List[str], ctx: RouteCodeContext):
    from .. import __version__

    ctx.console.print(f"[accent]routecode[/accent] [white]{__version__}[/white]")
    from . import COMMANDS

    ctx.console.print(f"[dim]Python based, {len(COMMANDS)} commands[/dim]")


async def handle_update(args: List[str], ctx: RouteCodeContext):
    from ..updater import check_for_update, perform_update

    with ctx.console.status("[accent]Checking for updates...[/accent]", spinner="dots"):
        info = check_for_update()

    if info.error:
        ctx.console.print(f"[dim]Could not check for updates: {info.error}[/dim]")
        ctx.console.print(
            f"[dim]Visit [link={info.release_url}]{info.release_url}[/link][/dim]"
        )
        return

    if not info.is_available:
        short_ver = info.current_version.split("+")[0].split(".dev")[0]
        ctx.console.print(
            f"[success]RouteCode is up to date[/success] [dim]({short_ver})[/dim]"
        )
        return

    ctx.console.print(
        f"[accent]Update available![/accent] "
        f"[white]{info.current_version}[/white] → [white]{info.latest_version}[/white]"
    )
    ctx.console.print(f"[dim]{info.install_type} install[/dim]")

    perform_update(info, console=ctx.console)


async def handle_exit(args: List[str], ctx: RouteCodeContext):
    from .session import handle_save

    await handle_save([], ctx)
    raise EOFError("exit")
