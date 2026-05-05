import os
import json
from typing import Any, List, Dict, Optional
from rich.console import Console, Group
from rich.theme import Theme
from rich.panel import Panel
from rich.columns import Columns
from rich.text import Text
from rich.layout import Layout
from rich.align import Align
from rich.box import ROUNDED, MINIMAL, SIMPLE, SQUARE, HEAVY
from rich.table import Table
from rich.progress import Progress, SpinnerColumn, BarColumn, TextColumn, TimeElapsedColumn
from rich.markdown import Markdown
from rich.syntax import Syntax
from rich.rule import Rule
from .state import state

# Background color per theme (hex)
THEME_BACKGROUNDS = {
    "lava":     "#1a1a2e",
    "ocean":    "#0d1b2a",
    "forest":   "#0b1a0b",
    "sunset":   "#1f1008",
    "midnight": "#0a0a14",
}

THEMES = {
    "lava": {
        "info": "bright_black", "warning": "yellow", "error": "bold red",
        "success": "bold green", "prompt": "bold white", "command": "bold blue",
        "accent": "red1", "dim": "bright_black", "border": "bright_black",
        "title": "bold white", "toolbar": "white on grey15",
        "user": "bold white", "ai": "bold white", "thought": "italic bright_black",
        "tool": "red1", "tool_bash": "bold cyan", "tool_read": "bold blue",
        "tool_edit": "bold yellow", "tool_write": "bold magenta",
        "tool_glob": "bold green", "tool_grep": "bold green",
        "tool_task": "bold white", "tool_skill": "bold white",
        "tool_webfetch": "bold cyan", "stats_label": "bright_black",
        "stats_value": "white",
    },
    "ocean": {
        "info": "bright_black", "warning": "yellow", "error": "bold red",
        "success": "bold green", "prompt": "bold white", "command": "bold blue",
        "accent": "deep_sky_blue1", "dim": "bright_black", "border": "bright_black",
        "title": "bold white", "toolbar": "white on grey15",
        "user": "bold white", "ai": "bold white", "thought": "italic bright_black",
        "tool": "deep_sky_blue1", "tool_bash": "bold cyan", "tool_read": "bold blue",
        "tool_edit": "bold yellow", "tool_write": "bold magenta",
        "tool_glob": "bold green", "tool_grep": "bold green",
        "tool_task": "bold white", "tool_skill": "bold white",
        "tool_webfetch": "bold cyan", "stats_label": "bright_black",
        "stats_value": "white",
    },
    "forest": {
        "info": "bright_black", "warning": "yellow", "error": "bold red",
        "success": "bold green", "prompt": "bold white", "command": "bold blue",
        "accent": "green3", "dim": "bright_black", "border": "bright_black",
        "title": "bold white", "toolbar": "white on grey15",
        "user": "bold white", "ai": "bold white", "thought": "italic bright_black",
        "tool": "green3", "tool_bash": "bold cyan", "tool_read": "bold blue",
        "tool_edit": "bold yellow", "tool_write": "bold magenta",
        "tool_glob": "bold green", "tool_grep": "bold green",
        "tool_task": "bold white", "tool_skill": "bold white",
        "tool_webfetch": "bold cyan", "stats_label": "bright_black",
        "stats_value": "white",
    },
    "sunset": {
        "info": "bright_black", "warning": "yellow", "error": "bold red",
        "success": "bold green", "prompt": "bold white", "command": "bold blue",
        "accent": "dark_orange", "dim": "bright_black", "border": "bright_black",
        "title": "bold white", "toolbar": "white on grey15",
        "user": "bold white", "ai": "bold white", "thought": "italic bright_black",
        "tool": "dark_orange", "tool_bash": "bold cyan", "tool_read": "bold blue",
        "tool_edit": "bold yellow", "tool_write": "bold magenta",
        "tool_glob": "bold green", "tool_grep": "bold green",
        "tool_task": "bold white", "tool_skill": "bold white",
        "tool_webfetch": "bold cyan", "stats_label": "bright_black",
        "stats_value": "white",
    },
    "midnight": {
        "info": "bright_black", "warning": "yellow", "error": "bold red",
        "success": "bold green", "prompt": "bold white", "command": "bold cyan",
        "accent": "medium_purple", "dim": "bright_black", "border": "bright_black",
        "title": "bold white", "toolbar": "bright_black on grey3",
        "user": "bold white", "ai": "bold white", "thought": "italic bright_black",
        "tool": "medium_purple", "tool_bash": "bold cyan", "tool_read": "bold blue",
        "tool_edit": "bold yellow", "tool_write": "bold magenta",
        "tool_glob": "bold green", "tool_grep": "bold green",
        "tool_task": "bold white", "tool_skill": "bold white",
        "tool_webfetch": "bold cyan", "stats_label": "bright_black",
        "stats_value": "white",
    },
}


_current_theme_name = "lava"


def get_theme_bg(name=None):
    """Return the hex background color for the given (or current) theme."""
    return THEME_BACKGROUNDS.get(name or _current_theme_name, "#1a1a2e")


def _set_terminal_bg(hex_color):
    """Paint the entire terminal background using ANSI true-color sequences."""
    r = int(hex_color[1:3], 16)
    g = int(hex_color[3:5], 16)
    b = int(hex_color[5:7], 16)
    import sys
    # OSC 11 sets the terminal default background (most modern terminals)
    sys.stdout.write(f"\033]11;#{r:02x}{g:02x}{b:02x}\007")
    # SGR 48;2 sets background for subsequent text
    sys.stdout.write(f"\033[48;2;{r};{g};{b}m")
    sys.stdout.flush()


def apply_theme(name: str = "lava"):
    global _current_theme_name, loom_theme, console
    _current_theme_name = name
    theme_dict = THEMES.get(name, THEMES["lava"])
    loom_theme = Theme(theme_dict)

    bg = get_theme_bg(name)
    _set_terminal_bg(bg)

    # Recreate the console so every Rich print inherits the background
    console = Console(theme=loom_theme, style=f"on {bg}")

    # Patch sys.stdout.write to ensure all scrolled/new lines explicitly paint the rest of the line
    import sys
    if not hasattr(sys.stdout, "_bg_fill_patched"):
        _original_write = sys.stdout.write
        def _bg_fill_write(s):
            current_bg = get_theme_bg()
            if current_bg and isinstance(s, str):
                try:
                    r, g, b = int(current_bg[1:3], 16), int(current_bg[3:5], 16), int(current_bg[5:7], 16)
                    bg_seq = f"\033[48;2;{r};{g};{b}m"
                    
                    # 1. Force rich's animation clear sequences to use the background color
                    if "\x1b[2K" in s:
                        s = s.replace("\x1b[2K", f"{bg_seq}\x1b[2K")
                    if "\x1b[K" in s:
                        s = s.replace("\x1b[K", f"{bg_seq}\x1b[K")
                        
                    # 2. Force newlines to clear the rest of the line with the background color
                    if "\n" in s:
                        s = s.replace("\n", f"{bg_seq}\x1b[K\n")
                except Exception:
                    pass
            _original_write(s)
        sys.stdout.write = _bg_fill_write
        sys.stdout._bg_fill_patched = True


loom_theme = Theme(THEMES["lava"])
console = Console(theme=loom_theme, style=f"on {THEME_BACKGROUNDS['lava']}")

TOOL_STYLES = {
    "bash": "tool_bash",
    "file_read": "tool_read",
    "file_edit": "tool_edit",
    "file_write": "tool_write",
    "glob": "tool_glob",
    "grep": "tool_grep",
    "task": "tool_task",
    "skill": "tool_skill",
    "webfetch": "tool_webfetch",
}


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
    """An animated face that cycles through expressions each render."""
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
            # For the main Thinking... indicator
            content = Group(
                Text(f" {self.face}\n", style="accent"),
                self.progress
            )
            yield Panel(content, border_style=f"on {bg}", style=panel_style, padding=(0, 1))

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


def print_welcome_screen(user_name: str, model: str, provider: str):
    logo = get_logo()

    content = Text()
    content.append(Text.from_markup(f"[accent]{logo}[/accent]"))
    content.append("\n\n")

    welcome = Text.from_markup(
        f"[dim]Welcome, [/dim][white]{user_name}[/white]\n\n"
        f"[accent]Build[/accent] · [white]{model}[/white] [dim]{provider}[/dim]"
    )
    content.append(welcome)
    content.append("\n\n")

    shortcuts = Table.grid(padding=(0, 2))
    shortcuts.add_column()
    shortcuts.add_column()
    shortcuts.add_row("[dim]/help[/dim]", "[dim]Show commands[/dim]")
    shortcuts.add_row("[dim]/tools[/dim]", "[dim]List tools[/dim]")
    shortcuts.add_row("[dim]/provider[/dim]", "[dim]Switch provider[/dim]")
    shortcuts.add_row("[dim]/tasks[/dim]", "[dim]View tasks[/dim]")

    tip_box = Panel(
        shortcuts,
        title="[dim]Quick Start[/dim]",
        border_style="dim",
        padding=(1, 2)
    )

    layout = Layout()
    layout.split_column(
        Layout(Align.center(content, vertical="middle"), ratio=1),
        Layout(name="footer", size=1)
    )
    footer_text = Text(f"{os.getcwd()}", style="dim")
    layout["footer"].update(Align.left(footer_text, vertical="bottom"))

    h = 24
    try:
        from prompt_toolkit.output.defaults import create_output
        h = create_output().get_size().rows
    except Exception:
        import shutil
        h = shutil.get_terminal_size().lines
    
    # tip_box takes 8 lines. The prompt and toolbar take 2 lines.
    # Total printed lines before the prompt should be h - 2.
    # So layout height should be (h - 2) - 8 = h - 10.
    console.print(layout, height=max(15, h - 10))
    console.print(tip_box)


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
    if detail:
        label = f"{verb} {truncate(detail)}"
    else:
        others = {k: truncate(v, 40) for k, v in arguments.items() if k != "file_path"}
        extras = f" {others}" if others else ""
        label = f"{verb}{extras}"
    return label


def print_tool_call(name: str, arguments: dict):
    label = get_tool_label(name, arguments)
    style = TOOL_STYLES.get(name, "tool")
    console.print(f" [{style}]>[/] [{style}]{label}[/]")


def print_tool_result(result: Any, duration: float = 0.0, tool_name: str = ""):
    style = TOOL_STYLES.get(tool_name, "dim")
    if isinstance(result, dict) and result.get("success"):
        msg = result.get("message", "Success")
        stats_str = ""
        if "stats" in result:
            stats = result["stats"]
            added = stats.get("added", 0)
            removed = stats.get("removed", 0)
            if added > 0 or removed > 0:
                stats_str = f" [green]+{added}[/green] [red]-{removed}[/red]"
        time_str = f" [dim]({duration:.1f}s)[/dim]" if duration >= 0.5 else ""
        console.print(f"   [success]✔[/success] [{style}]{msg}{stats_str}{time_str}[/]")
    elif isinstance(result, dict) and "error" in result:
        console.print(f"   [error]✘[/error] [error]{result['error']}[/error]")
    else:
        res_str = str(result)[:200] + "... [truncated]" if len(str(result)) > 200 else str(result)
        console.print(f"   [{style}]Result: {res_str}[/]")


def print_session_stats():
    table = Table(show_header=False, box=MINIMAL, padding=(0, 1))
    table.add_column()
    table.add_column()
    table.add_row("[stats_label]Context[/stats_label]", f"{state.tokens_used:,} tokens")
    table.add_row("[stats_label]Cost[/stats_label]", f"${state.estimated_cost:.2f}")
    table.add_row("[stats_label]Session[/stats_label]", f"{int(time.time() - state.start_time)}s" if state.start_time else "0s")
    console.print(table)


import time


def print_info(message: str):
    console.print(f"[info]ℹ[/info] {message}")


def print_success(message: str):
    console.print(f"[success]✔[/success] {message}")


def print_warning(message: str):
    console.print(f"[warning]⚠[/warning] {message}")


def print_error(message: str):
    console.print(f"[error]✘[/error] {message}")


def print_step(message: str):
    console.print(f"[accent]➤[/accent] {message}")


def print_diff(diff_lines: list, max_lines: int = 30):
    if not diff_lines:
        return
    text = Text()
    truncated = len(diff_lines) > max_lines
    display = diff_lines[:max_lines]
    for line in display:
        if line.startswith('+') and not line.startswith('+++'):
            text.append(line + '\n', style="green")
        elif line.startswith('-') and not line.startswith('---'):
            text.append(line + '\n', style="red")
        elif line.startswith('@@'):
            text.append(line + '\n', style="cyan")
        else:
            text.append(line + '\n', style="dim")
    if truncated:
        text.append(f"... ({len(diff_lines) - max_lines} more lines)", style="dim")
    console.print(Panel(text, title="[bold]Diff[/bold]", border_style="green", padding=(0, 1)))
