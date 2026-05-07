import os
from typing import Any, Optional
from rich.console import Group, Console
from rich.panel import Panel
from rich.text import Text
from rich.layout import Layout
from rich.align import Align
from rich.table import Table
from rich.progress import Progress, SpinnerColumn, BarColumn, TextColumn, TimeElapsedColumn
from rich.markdown import Markdown
from .console import console
from .theme import get_theme_bg

def get_logo():
    return """
██╗      ██████╗  ██████╗ ███╗   ███╗
██║     ██╔═══██╗██╔═══██╗████╗ ████║
██║     ██║   ██║██║   ██║██╔████╔██║
██║     ██║   ██║██║   ██║██║╚██╔╝██║
███████╗╚██████╔╝╚██████╔╝██║ ╚═╝ ██║
╚══════╝ ╚═════╝  ╚═════╝ ╚═╝     ╚═╝
""".strip()

LOOM_FACES = [
    " (o.o)  LOOM  (o.o) ",
    " (-.-)  LOOM  (-.-) ",
    " (o.o)  LOOM  (o.o) ",
    " (O.O)  LOOM  (O.O) ",
    " (o.o)  LOOM  (o.o) ",
    " (^_^)  LOOM  (^_^) ",
    " (o.o)  LOOM  (o.o) ",
    " (-.-)  LOOM  (-.-) ",
    " (o.o)  LOOM  (o.o) ",
    " (>.<)  LOOM  (>.<) ",
]

class LoomFace:
    def __init__(self):
        self._index = 0
    def __rich__(self):
        frame = LOOM_FACES[self._index % len(LOOM_FACES)]
        self._index += 1
        return Text(frame, style="accent")
    def __str__(self):
        frame = LOOM_FACES[self._index % len(LOOM_FACES)]
        self._index += 1
        return frame

class LoadingRenderable:
    def __init__(self, progress: Progress, markdown: Optional[Markdown] = None,
                 thought: Optional[str] = None, info: Optional[str] = None,
                 elapsed: float = 0.0):
        self.progress = progress
        self.markdown = markdown
        self.thought = thought
        self.info = info
        self.elapsed = elapsed
        self.face = LoomFace()

    def __rich_console__(self, console, options):
        bg = get_theme_bg()
        panel_style = f"on {bg}"
        if self.info:
            yield Panel(Text(self.info, style="dim"), border_style="dim", padding=(0, 1), style=panel_style)
        elif self.markdown and self.markdown.markup:
            yield Panel(self.markdown, border_style="dim", style=panel_style)
        elif self.thought:
            yield Panel(
                Text(f" {self.face}  {self.thought}", style="thought"),
                border_style="dim", padding=(0, 1), style=panel_style
            )
        else:
            content = Group(
                Text(f" {self.face}\n", style="accent"),
                self.progress
            )
            yield Panel(content, border_style=f"on {bg}", style=panel_style, padding=(0, 1))

def refresh_screen(ctx):
    import sys, getpass
    from ..utils import parse_hex_color
    bg = get_theme_bg()
    r, g, b = parse_hex_color(bg)
    sys.stdout.write(f"\033[48;2;{r};{g};{b}m\033[2J\033[H")
    sys.stdout.flush()
    print_welcome_screen(getpass.getuser(), ctx.config.model, ctx.config.provider)

def get_thinking_indicator():
    progress = Progress(
        SpinnerColumn(spinner_name="dots", style="accent"),
        TextColumn("[thought]Thinking...[/thought]"),
        BarColumn(bar_width=20, style="dim", complete_style="accent", finished_style="accent"),
        TimeElapsedColumn(),
        transient=True,
        console=console
    )
    progress.add_task("thinking", total=None)
    return progress

def print_welcome_screen(user_name: str, model: str, provider: str, target_console: Optional[Console] = None):
    c = target_console or console
    logo = get_logo()
    content = Text()
    content.append(Text.from_markup(f"[accent]{logo}[/accent]"))
    content.append("\n\n")
    welcome = Text.from_markup(f"[dim]Welcome, [/dim][white]{user_name}[/white]\n\n[accent]Build[/accent] · [white]{model}[/white] [dim]{provider}[/dim]")
    content.append(welcome)
    content.append("\n\n")
    shortcuts = Table.grid(padding=(0, 2))
    shortcuts.add_column(); shortcuts.add_column()
    shortcuts.add_row("[dim]/help[/dim]", "[dim]Show commands[/dim]")
    shortcuts.add_row("[dim]/tools[/dim]", "[dim]List tools[/dim]")
    shortcuts.add_row("[dim]/provider[/dim]", "[dim]Switch provider[/dim]")
    shortcuts.add_row("[dim]/tasks[/dim]", "[dim]View tasks[/dim]")
    tip_box = Panel(shortcuts, title="[dim]Quick Start[/dim]", border_style="dim", padding=(1, 2))
    # Just print content directly for the new full-screen REPL
    c.print(Align.center(content))
    c.print(tip_box)

def print_thought_elapsed(elapsed: float):
    console.print(f"\n[thought]Thought for {elapsed:.1f}s[/thought]")

def print_status_line(model: str, elapsed: Optional[float] = None):
    time_str = f" · {elapsed:.1f}s" if elapsed is not None else ""
    console.print(f" [accent]■[/accent] [white]Build[/white] · [dim]{model}{time_str}[/dim]")

def get_tool_label(name: str, arguments: dict) -> str:
    def truncate(v, n=80):
        s = str(v)
        return s[:n] + "..." if len(s) > n else s
    verbs = {
        "file_read": ("Read", arguments.get("file_path", "")),
        "file_write": ("Write", arguments.get("file_path", "")),
        "file_edit": ("Edit", arguments.get("file_path", "")),
        "bash": ("Run", arguments.get("command", "")),
        "glob": ("List", arguments.get("pattern", "")),
        "grep": ("Search", arguments.get("pattern", "")),
        "task": ("Task", arguments.get("task", "")),
        "skill": ("Skill", arguments.get("skill", "")),
        "webfetch": ("Fetch", arguments.get("url", "")),
    }
    verb, detail = verbs.get(name, (name, ""))
    label = f"{verb} {truncate(detail)}" if detail else f"{verb} { {k: truncate(v, 40) for k, v in arguments.items() if k != 'file_path'} }"
    return label

def print_tool_call(name: str, arguments: dict):
    from .theme import THEMES, _current_theme_name
    label = get_tool_label(name, arguments)
    style = "accent" # Placeholder style lookup logic if needed, or use simple
    console.print(f" [accent]>[/] [accent]{label}[/]")

def print_tool_result(result: Any, duration: float = 0.0, tool_name: str = ""):
    if isinstance(result, dict) and result.get("success"):
        msg = result.get("message", "Success")
        stats_str = ""
        if "stats" in result:
            stats = result["stats"]
            added = stats.get("added", 0); removed = stats.get("removed", 0)
            if added > 0 or removed > 0: stats_str = f" [green]+{added}[/green] [red]-{removed}[/red]"
        time_str = f" [dim]({duration:.1f}s)[/dim]" if duration >= 0.5 else ""
        console.print(f"   [success]✔[/success] [dim]{msg}{stats_str}{time_str}[/]")
    elif isinstance(result, dict) and "error" in result:
        console.print(f"   [error]✘[/error] [error]{result['error']}[/error]")
    else:
        res_str = str(result)[:200] + "... [truncated]" if len(str(result)) > 200 else str(result)
        console.print(f"   [dim]Result: {res_str}[/]")

def print_session_stats(state):
    from rich.box import MINIMAL
    import time
    table = Table(show_header=False, box=MINIMAL, padding=(0, 1))
    table.add_column(); table.add_column()
    table.add_row("[stats_label]Context[/stats_label]", f"{state.tokens_used:,} tokens")
    table.add_row("[stats_label]Cost[/stats_label]", f"${state.estimated_cost:.2f}")
    table.add_row("[stats_label]Session[/stats_label]", f"{int(time.time() - state.start_time)}s" if state.start_time else "0s")
    console.print(table)

def print_diff(diff_lines: list, max_lines: int = 30):
    if not diff_lines: return
    text = Text()
    truncated = len(diff_lines) > max_lines
    display = diff_lines[:max_lines]
    for line in display:
        if line.startswith('+') and not line.startswith('+++'): text.append(line + '\n', style="green")
        elif line.startswith('-') and not line.startswith('---'): text.append(line + '\n', style="red")
        elif line.startswith('@@'): text.append(line + '\n', style="cyan")
        else: text.append(line + '\n', style="dim")
    if truncated: text.append(f"... ({len(diff_lines) - max_lines} more lines)", style="dim")
    console.print(Panel(text, title="[bold]Diff[/bold]", border_style="green", padding=(0, 1)))
