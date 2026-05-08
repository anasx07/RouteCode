from typing import Any, Optional
from rich.console import Group, Console
from rich.panel import Panel
from rich.text import Text
from rich.align import Align
from rich.table import Table
from rich.progress import (
    Progress,
    SpinnerColumn,
    BarColumn,
    TextColumn,
    TimeElapsedColumn,
)
from rich.markdown import Markdown
from rich.spinner import Spinner
from rich.columns import Columns
from .console import console
from .theme import get_theme_bg


def get_logo():
    # Pure ASCII art — renders correctly in every terminal and font
    return (
        "##        ######    ######   ##    ##\n"
        "##       ##    ##  ##    ##  ###  ###\n"
        "##       ##    ##  ##    ##  ## ## ##\n"
        "##       ##    ##  ##    ##  ##    ##\n"
        "########  ######    ######   ##    ##"
    )


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
    def __init__(
        self,
        progress: Progress,
        markdown: Optional[Markdown] = None,
        thought: Optional[str] = None,
        info: Optional[str] = None,
        elapsed: float = 0.0,
    ):
        self.progress = progress
        self.markdown = markdown
        self.thought = thought
        self.info = info
        self.elapsed = elapsed
        self.face = Spinner("dots", style="accent")

    def __rich_console__(self, console, _options):
        bg = get_theme_bg()
        panel_style = f"on {bg}"
        if self.info:
            yield Panel(
                Text(self.info, style="dim"),
                border_style="dim",
                padding=(0, 1),
                style=panel_style,
            )
        elif self.markdown and self.markdown.markup:
            yield Panel(self.markdown, border_style="dim", style=panel_style)
        elif self.thought:
            yield Panel(
                Columns([self.face, Text(f" {self.thought}", style="thought")]),
                border_style="dim",
                padding=(0, 1),
                style=panel_style,
            )
        else:
            content = Group(self.face, self.progress)
            yield Panel(
                content, border_style=f"on {bg}", style=panel_style, padding=(0, 1)
            )


def refresh_screen(ctx):
    import sys
    import getpass
    from ..utils.helpers import parse_hex_color

    bg = get_theme_bg()
    r, g, b = parse_hex_color(bg)
    sys.stdout.write(f"\033[48;2;{r};{g};{b}m\033[2J\033[H")
    sys.stdout.flush()
    print_welcome_screen(getpass.getuser(), ctx.config.model, ctx.config.provider)


def get_thinking_indicator():
    progress = Progress(
        SpinnerColumn(spinner_name="dots", style="accent"),
        TextColumn("[thought]Thinking...[/thought]"),
        BarColumn(
            bar_width=20, style="dim", complete_style="accent", finished_style="accent"
        ),
        TimeElapsedColumn(),
        transient=True,
        console=console,
    )
    progress.add_task("thinking", total=None)
    return progress


def print_welcome_screen(
    user_name: str, model: str, provider: str, target_console: Optional[Console] = None
):
    c = target_console or console
    logo = get_logo()
    from rich.box import ROUNDED

    # ── Logo block ──────────────────────────────────────────────────────────
    logo_text = Text()
    logo_text.append("\n")
    logo_text.append(Text.from_markup(f"[accent]{logo}[/accent]"))
    logo_text.append("\n")
    c.print(Align.center(logo_text))

    # ── Identity / model line ────────────────────────────────────────────────
    identity = Text.from_markup(
        f"[dim]Welcome back,[/dim] [bold white]{user_name}[/bold white]"
    )
    build_line = Text.from_markup(
        f"[accent]Build[/accent]  [dim]·[/dim]  [bold white]{model}[/bold white]  [dim]({provider})[/dim]"
    )
    c.print(Align.center(identity))
    c.print(Align.center(build_line))
    c.print()

    # ── Quick-start panel ────────────────────────────────────────────────────
    shortcuts = Table.grid(padding=(0, 3))
    shortcuts.add_column(justify="right", min_width=10)
    shortcuts.add_column(justify="left")
    shortcuts.add_row(
        "[accent bold]/help[/accent bold]", "[dim]List all commands[/dim]"
    )
    shortcuts.add_row(
        "[accent bold]/tools[/accent bold]", "[dim]Manage available tools[/dim]"
    )
    shortcuts.add_row(
        "[accent bold]/provider[/accent bold]", "[dim]Switch AI provider[/dim]"
    )
    shortcuts.add_row(
        "[accent bold]/tasks[/accent bold]", "[dim]Track background tasks[/dim]"
    )

    tip_box = Panel(
        Align.center(shortcuts),
        title="[dim]  Quick Start  [/dim]",
        border_style="bright_black",
        padding=(0, 4),
        expand=False,
        box=ROUNDED,
    )
    c.print(Align.center(tip_box))
    c.print()


def print_thought_elapsed(elapsed: float):
    console.print(f"\n[thought]Thought for {elapsed:.1f}s[/thought]")


def print_status_line(model: str, elapsed: Optional[float] = None):
    time_str = f" · {elapsed:.1f}s" if elapsed is not None else ""
    console.print(
        f" [accent]■[/accent] [white]Build[/white] · [dim]{model}{time_str}[/dim]"
    )


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
    label = (
        f"{verb} {truncate(detail)}"
        if detail
        else f"{verb} { {k: truncate(v, 40) for k, v in arguments.items() if k != 'file_path'} }"
    )
    return label


def format_duration(seconds: float) -> str:
    if seconds < 1:
        return f"{seconds:.1f}s"
    seconds = int(seconds)
    if seconds < 60:
        return f"{seconds}s"
    m = seconds // 60
    s = seconds % 60
    return f"{m}m {s}s"


def print_user_message(text: str, target_console: Optional[Console] = None):
    c = target_console or console
    from rich.padding import Padding
    
    c.print("\n [accent]●[/accent] [bold white]You[/bold white]")
    
    msg_text = Text(text, style="white")
    padded = Padding(msg_text, (0, 0, 0, 3))
    c.print(padded)
    c.print()


def print_tool_call(name: str, arguments: dict):
    label = get_tool_label(name, arguments)
    console.print(f" [accent]>[/] [accent]{label}[/]")


def print_tool_result(result: Any, duration: float = 0.0, _tool_name: str = ""):
    time_str = f" [dim](Worked for {format_duration(duration)})[/dim]" if duration > 0.0 else ""
    if isinstance(result, dict) and result.get("success"):
        msg = result.get("message", "Success")
        stats_str = ""
        if "stats" in result:
            stats = result["stats"]
            added = stats.get("added", 0)
            removed = stats.get("removed", 0)
            if added > 0 or removed > 0:
                stats_str = f" [green]+{added}[/green] [red]-{removed}[/red]"
        console.print(f"   [success]✔[/success] [dim]{msg}{stats_str}{time_str}[/]")
    elif isinstance(result, dict) and "error" in result:
        console.print(f"   [error]✘[/error] [error]{result['error']}[/error]{time_str}")
    else:
        res_str = (
            str(result)[:200] + "... [truncated]"
            if len(str(result)) > 200
            else str(result)
        )
        console.print(f"   [dim]Result: {res_str}{time_str}[/]")


def print_session_stats(state):
    from rich.box import MINIMAL
    import time

    table = Table(show_header=False, box=MINIMAL, padding=(0, 1))
    table.add_column()
    table.add_column()
    table.add_row("[stats_label]Context[/stats_label]", f"{state.tokens_used:,} tokens")
    table.add_row("[stats_label]Cost[/stats_label]", f"${state.estimated_cost:.2f}")
    table.add_row(
        "[stats_label]Session[/stats_label]",
        f"{int(time.time() - state.start_time)}s" if state.start_time else "0s",
    )
    console.print(table)


def print_diff(diff_lines: list, max_lines: int = 30):
    if not diff_lines:
        return
    text = Text()
    truncated = len(diff_lines) > max_lines
    display = diff_lines[:max_lines]
    for line in display:
        if line.startswith("+") and not line.startswith("+++"):
            text.append(line + "\n", style="green")
        elif line.startswith("-") and not line.startswith("---"):
            text.append(line + "\n", style="red")
        elif line.startswith("@@"):
            text.append(line + "\n", style="cyan")
        else:
            text.append(line + "\n", style="dim")
    if truncated:
        text.append(f"... ({len(diff_lines) - max_lines} more lines)", style="dim")
    console.print(
        Panel(text, title="[bold]Diff[/bold]", border_style="green", padding=(0, 1))
    )
