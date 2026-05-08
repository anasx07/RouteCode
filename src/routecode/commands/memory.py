from typing import List
from rich.table import Table
from ..ui import print_success, print_error
from ..core import RouteCodeContext


async def handle_remember(args: List[str], ctx: RouteCodeContext):
    if not args:
        print_error("Usage: /remember <key> <value>")
        return
    key = args[0]
    value = " ".join(args[1:])
    if not value:
        print_error("Usage: /remember <key> <value>")
        return
    msg = ctx.memory.remember(key, value)
    await ctx.memory._save_async()
    print_success(msg)


async def handle_forget(args: List[str], ctx: RouteCodeContext):
    if not args:
        print_error("Usage: /forget <key>")
        return
    msg = ctx.memory.forget(args[0])
    await ctx.memory._save_async()
    ctx.console.print(f" [dim]{msg}[/dim]")


def handle_memories(args: List[str], ctx: RouteCodeContext):
    memories = ctx.memory.list()
    if not memories:
        ctx.console.print(
            "[dim]No memories saved yet. Use /remember <key> <value> to save one.[/dim]"
        )
        return
    table = Table(title="Session Memory", show_header=True, header_style="bold cyan")
    table.add_column("Key", style="bold yellow")
    table.add_column("Value")
    for key, value in memories.items():
        table.add_row(key, value[:80])
    ctx.console.print(table)
