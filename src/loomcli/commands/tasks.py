import time
from typing import List
from rich.table import Table
from ..ui import print_success, print_error
from ..core import LoomContext


def handle_tasks(args: List[str], ctx: LoomContext):
    tasks = ctx.task_manager.list()
    if not tasks:
        ctx.console.print("[dim]No active or recent tasks.[/dim]")
        return
    table = Table(title="Tasks", show_header=True, header_style="bold cyan")
    table.add_column("ID", style="bold yellow")
    table.add_column("Description")
    table.add_column("Status", style="bold")
    table.add_column("Age")
    for t in tasks:
        status_style = {
            "running": "bold green",
            "completed": "dim",
            "failed": "bold red",
            "killed": "dim",
        }.get(t["status"], "")
        elapsed = time.time() - t["created_at"]
        age = f"{elapsed:.0f}s" if elapsed < 120 else f"{elapsed / 60:.0f}m"
        table.add_row(
            t["task_id"],
            t["description"][:60],
            f"[{status_style}]{t['status']}[/{status_style}]",
            age,
        )
    ctx.console.print(table)


def handle_task_stop(args: List[str], ctx: LoomContext):
    if not args:
        print_error("Usage: /task-stop <task_id>")
        return
    task_id = args[0]
    if ctx.task_manager.kill(task_id):
        print_success(f"Task {task_id} stopped.")
    else:
        print_error(f"Task {task_id} not found or not running.")
