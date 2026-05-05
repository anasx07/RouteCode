import os
import sys
import json
import atexit
import signal

def _cleanup_terminal():
    """Ensure terminal is left in a clean state (e.g., mouse tracking disabled)."""
    try:
        sys.stdout.write("\033[?1003l")
        sys.stdout.flush()
    except Exception:
        pass

atexit.register(_cleanup_terminal)

# Handle termination signals to ensure cleanup
def _signal_handler(signum, frame):
    _cleanup_terminal()
    sys.exit(signum)

try:
    signal.signal(signal.SIGTERM, _signal_handler)
except (AttributeError, ValueError):
    # Some platforms or environments don't support SIGTERM
    pass

import shutil
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
from prompt_toolkit.styles import Style
from prompt_toolkit.layout.containers import Window, HSplit, VSplit, FloatContainer, Float, WindowAlign, Container
from prompt_toolkit.layout.controls import FormattedTextControl
from prompt_toolkit.layout.layout import Layout as PTLayout
from prompt_toolkit.widgets import RadioList, Button, Dialog, Label
from prompt_toolkit.application import Application
from prompt_toolkit.key_binding import KeyBindings
from prompt_toolkit.formatted_text import ANSI
from prompt_toolkit.output.defaults import create_output
from prompt_toolkit.layout.menus import CompletionsMenu, CompletionsMenuControl
from prompt_toolkit.mouse_events import MouseEvent, MouseEventType
import io


class HoverCompletionsMenuControl(CompletionsMenuControl):
    """CompletionsMenuControl with MOUSE_MOVE hover support."""
    
    def mouse_handler(self, mouse_event: MouseEvent):
        b = get_app().current_buffer

        if mouse_event.event_type == MouseEventType.MOUSE_MOVE:
            # Hover: highlight the completion under the cursor
            b.go_to_completion(mouse_event.position.y)

        elif mouse_event.event_type == MouseEventType.MOUSE_UP:
            # Click: select and accept
            b.go_to_completion(mouse_event.position.y)
            b.complete_state = None

        elif mouse_event.event_type == MouseEventType.SCROLL_DOWN:
            b.complete_next(count=3, disable_wrap_around=True)

        elif mouse_event.event_type == MouseEventType.SCROLL_UP:
            b.complete_previous(count=3, disable_wrap_around=True)

        return None


def _get_app():
    """Lazy import to avoid circular imports."""
    from prompt_toolkit.application.current import get_app
    return get_app()

# Patch the module-level get_app for HoverCompletionsMenuControl
from prompt_toolkit.application.current import get_app


class HoverCompletionsMenu(CompletionsMenu):
    """A CompletionsMenu that highlights items on mouse hover."""
    
    def __init__(self, max_height=None, scroll_offset=0, extra_filter=True, 
                 display_arrows=False, z_index=10**8):
        from prompt_toolkit.filters import to_filter, has_completions, is_done
        from prompt_toolkit.layout.dimension import Dimension
        from prompt_toolkit.layout.margins import ScrollbarMargin, ConditionalMargin
        from prompt_toolkit.filters import Condition
        
        extra_filter = to_filter(extra_filter)
        display_arrows = to_filter(display_arrows)
        
        # Use our hover-aware control instead of the standard one
        super(CompletionsMenu, self).__init__(
            content=Window(
                content=HoverCompletionsMenuControl(),
                width=Dimension(min=8),
                height=Dimension(min=1, max=max_height),
                scroll_offsets=ScrollOffsets(top=scroll_offset, bottom=scroll_offset),
                right_margins=[ScrollbarMargin(display_arrows=display_arrows)],
                dont_extend_width=True,
                style="class:completion-menu",
                z_index=z_index,
            ),
            filter=extra_filter & has_completions & ~is_done,
        )

class HoverRadioList(RadioList):
    """A RadioList that highlights items on mouse hover (MOUSE_MOVE), not just click."""
    
    def _get_text_fragments(self):
        """Override to add MOUSE_MOVE handling for true hover support."""
        from prompt_toolkit.mouse_events import MouseEvent, MouseEventType
        from prompt_toolkit.formatted_text import to_formatted_text
        
        def mouse_handler(mouse_event: MouseEvent) -> None:
            """Handle both hover (MOUSE_MOVE) and click (MOUSE_UP) per-row."""
            if mouse_event.event_type == MouseEventType.MOUSE_MOVE:
                # Hover: just move the highlight, don't select
                self._selected_index = mouse_event.position.y
            elif mouse_event.event_type == MouseEventType.MOUSE_UP:
                # Click: move highlight AND select
                self._selected_index = mouse_event.position.y
                self._handle_enter()

        result = []
        for i, value in enumerate(self.values):
            checked = value[0] == self.current_value
            selected = i == self._selected_index

            style = ""
            if checked:
                style += " " + self.checked_style
            if selected:
                style += " " + self.selected_style

            result.append((style, self.open_character))
            if selected:
                result.append(("[SetCursorPosition]", ""))
            if checked:
                result.append((style, self.select_character))
            else:
                result.append((style, " "))
            result.append((style, self.close_character))
            result.append((f"{style} {self.default_style}", " "))

            result.extend(
                to_formatted_text(value[1], style=f"{style} {self.default_style}")
            )
            result.append(("", "\n"))

        # Attach the hover-aware mouse handler to ALL fragments
        for i in range(len(result)):
            result[i] = (result[i][0], result[i][1], mouse_handler)

        if result:
            result.pop()  # Remove last newline
        return result


class LoomDialog:
    """A premium floating dialog that appears over a dimmed version of the current CLI."""
    def __init__(self, title: str, text: str, dialog_type: str = "radio", 
                 values: Optional[List[tuple]] = None, buttons: Optional[List[tuple]] = None,
                 default: str = "", password: bool = False):
        self.title = title
        self.text = text
        self.dialog_type = dialog_type
        self.values = values or []
        self.buttons_config = buttons or [("OK", "ok"), ("Cancel", "cancel")]
        self.default = default
        self.password = password
        self.result = None

    def _get_backdrop_ansi(self) -> str:
        """Captures the REAL terminal state from the mirror and dims it."""
        full_ansi = _get_mirror_ansi()
        # Take the last N lines based on terminal height
        h = create_output().get_size().rows
        lines = full_ansi.splitlines()
        recent_lines = lines[-h:]
        ansi_content = "\n".join(recent_lines)
        
        # Apply global dimming (ANSI 2)
        return f"\033[2m{ansi_content}\033[0m"

    def run(self):
        backdrop_ansi = self._get_backdrop_ansi()
        
        # Create dialog widgets
        kb = KeyBindings()
        @kb.add("c-c")
        @kb.add("escape")
        def _(event):
            event.app.exit()

        buttons = []
        for label, value in self.buttons_config:
            def handler(v=value):
                self.result = v
                app.exit()
            buttons.append(Button(label, handler=handler))

        content = None
        if self.dialog_type == "radio":
            self.radio_list = HoverRadioList(self.values)
            content = HSplit([
                Label(self.text),
                self.radio_list
            ])
        elif self.dialog_type == "input":
            from prompt_toolkit.widgets import TextArea
            self.text_area = TextArea(text=self.default, password=self.password, multiline=False)
            content = HSplit([
                Label(self.text),
                self.text_area
            ])
            # Focus the text area by default
            @kb.add("enter")
            def _(event):
                self.result = "ok"
                app.exit()
        elif self.dialog_type == "button":
            content = Label(self.text)
        elif self.dialog_type == "message":
            content = Label(self.text)
            buttons = [Button("OK", handler=lambda: app.exit())]

        dialog = Dialog(
            title=self.title,
            body=content,
            buttons=buttons,
            with_background=True
        )

        # Create the layered layout
        root_container = FloatContainer(
            # The backdrop is just for display, it is non-focusable by default
            content=Window(content=FormattedTextControl(ANSI(backdrop_ansi)), align=WindowAlign.LEFT),
            floats=[
                Float(content=dialog)
            ]
        )

        # Force enable 'Any Event' mouse tracking for true hover support
        # \033[?1003h enables tracking of all mouse events, including movement.
        sys.stdout.write("\033[?1003h")
        sys.stdout.flush()

        app = Application(
            layout=PTLayout(root_container),
            key_bindings=kb,
            style=get_dialog_style(),
            full_screen=True,
            mouse_support=True
        )
        try:
            app.run()
        finally:
            # Disable any-event mouse tracking on exit to avoid garbage in the terminal
            sys.stdout.write("\033[?1003l")
            sys.stdout.flush()
        
        if self.dialog_type == "radio" and self.result == "ok":
            return self.radio_list.current_value
        if self.dialog_type == "input" and self.result == "ok":
            return self.text_area.text
        if self.dialog_type in ("radio", "input"):
            return None  # Cancel / Ctrl-C → return None, not the string "cancel"
        return self.result

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
        "accent": "#ff0000", "dim": "bright_black", "border": "bright_black",
        "title": "bold white", "toolbar": "white on grey15",
        "user": "bold white", "ai": "bold white", "thought": "italic bright_black",
        "tool": "#ff0000", "tool_bash": "bold cyan", "tool_read": "bold blue",
        "tool_edit": "bold yellow", "tool_write": "bold magenta",
        "tool_glob": "bold green", "tool_grep": "bold green",
        "tool_task": "bold white", "tool_skill": "bold white",
        "tool_webfetch": "bold cyan", "stats_label": "bright_black",
        "stats_value": "white",
    },
    "ocean": {
        "info": "bright_black", "warning": "yellow", "error": "bold red",
        "success": "bold green", "prompt": "bold white", "command": "bold blue",
        "accent": "#00afff", "dim": "bright_black", "border": "bright_black",
        "title": "bold white", "toolbar": "white on grey15",
        "user": "bold white", "ai": "bold white", "thought": "italic bright_black",
        "tool": "#00afff", "tool_bash": "bold cyan", "tool_read": "bold blue",
        "tool_edit": "bold yellow", "tool_write": "bold magenta",
        "tool_glob": "bold green", "tool_grep": "bold green",
        "tool_task": "bold white", "tool_skill": "bold white",
        "tool_webfetch": "bold cyan", "stats_label": "bright_black",
        "stats_value": "white",
    },
    "forest": {
        "info": "bright_black", "warning": "yellow", "error": "bold red",
        "success": "bold green", "prompt": "bold white", "command": "bold blue",
        "accent": "#00d700", "dim": "bright_black", "border": "bright_black",
        "title": "bold white", "toolbar": "white on grey15",
        "user": "bold white", "ai": "bold white", "thought": "italic bright_black",
        "tool": "#00d700", "tool_bash": "bold cyan", "tool_read": "bold blue",
        "tool_edit": "bold yellow", "tool_write": "bold magenta",
        "tool_glob": "bold green", "tool_grep": "bold green",
        "tool_task": "bold white", "tool_skill": "bold white",
        "tool_webfetch": "bold cyan", "stats_label": "bright_black",
        "stats_value": "white",
    },
    "sunset": {
        "info": "bright_black", "warning": "yellow", "error": "bold red",
        "success": "bold green", "prompt": "bold white", "command": "bold blue",
        "accent": "#ffaf00", "dim": "bright_black", "border": "bright_black",
        "title": "bold white", "toolbar": "white on grey15",
        "user": "bold white", "ai": "bold white", "thought": "italic bright_black",
        "tool": "#ffaf00", "tool_bash": "bold cyan", "tool_read": "bold blue",
        "tool_edit": "bold yellow", "tool_write": "bold magenta",
        "tool_glob": "bold green", "tool_grep": "bold green",
        "tool_task": "bold white", "tool_skill": "bold white",
        "tool_webfetch": "bold cyan", "stats_label": "bright_black",
        "stats_value": "white",
    },
    "midnight": {
        "info": "bright_black", "warning": "yellow", "error": "bold red",
        "success": "bold green", "prompt": "bold white", "command": "bold cyan",
        "accent": "#af87d7", "dim": "bright_black", "border": "bright_black",
        "title": "bold white", "toolbar": "bright_black on grey3",
        "user": "bold white", "ai": "bold white", "thought": "italic bright_black",
        "tool": "#af87d7", "tool_bash": "bold cyan", "tool_read": "bold blue",
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


def _set_terminal_bg(bg_color: str):
    """Sets the terminal background color using OSC 11 and palette color 0."""
    try:
        r, g, b = int(bg_color[1:3], 16), int(bg_color[3:5], 16), int(bg_color[5:7], 16)
        rgb_format = f"rgb:{r:02x}/{g:02x}/{b:02x}"
        
        sys.stdout.write(f"\033]11;rgb:{r:02x}/{g:02x}/{b:02x}\033\\")
        # Also set palette color 0 which many terminals use for margins
        sys.stdout.write(f"\033]4;0;rgb:{r:02x}/{g:02x}/{b:02x}\033\\")
        sys.stdout.flush()
    except Exception:
        pass


def apply_theme(name: str = "lava"):
    global _current_theme_name, loom_theme, console, mirror_console
    _current_theme_name = name
    theme_dict = THEMES.get(name, THEMES["lava"])
    loom_theme = Theme(theme_dict)

    bg = get_theme_bg(name)
    _set_terminal_bg(bg)

    # Recreate the console so every Rich print inherits the background
    console = Console(theme=loom_theme, style=f"on {bg}")
    
    # Also recreate mirror console to match new theme
    mirror_console = Console(theme=loom_theme, file=_mirror_output, force_terminal=True)
    
    # Re-patch the new console
    _orig_print = console.print
    def _mirrored_print(*args, **kwargs):
        _orig_print(*args, **kwargs)
        mirror_console.width = shutil.get_terminal_size().columns
        mirror_console.print(*args, **kwargs)
    console.print = _mirrored_print

    # Patch sys.stdout.write to ensure all scrolled/new lines explicitly paint the rest of the line
    if not hasattr(sys.stdout, "_bg_fill_patched"):
        _original_write = sys.stdout.write
        def _bg_fill_write(s):
            current_bg = get_theme_bg()
            if current_bg and isinstance(s, str):
                try:
                    r, g, b = int(current_bg[1:3], 16), int(current_bg[3:5], 16), int(current_bg[5:7], 16)
                    bg_seq = f"\033[48;2;{r};{g};{b}m"
                    
                    # 1. Force rich's animation clear sequences to use the background color
                    if "\x1b[2J" in s:
                        s = s.replace("\x1b[2J", f"{bg_seq}\x1b[2J")
                    if "\x1b[2K" in s:
                        s = s.replace("\x1b[2K", f"{bg_seq}\x1b[2K")
                    if "\x1b[K" in s:
                        s = s.replace("\x1b[K", f"{bg_seq}\x1b[K")
                    if "\x1b[J" in s:
                        s = s.replace("\x1b[J", f"{bg_seq}\x1b[J")
                        
                    # 2. Force newlines to clear the rest of the line with the background color
                    if "\n" in s:
                        s = s.replace("\n", f"{bg_seq}\x1b[K\n")
                except Exception:
                    pass
            _original_write(s)
        sys.stdout.write = _bg_fill_write
        sys.stdout._bg_fill_patched = True


def get_dialog_style():
    """Returns a dynamic prompt_toolkit Style for interactive dialogs based on current theme."""
    bg = get_theme_bg()
    theme = THEMES.get(_current_theme_name, THEMES["lava"])
    accent = theme.get("accent", "#ffaf00")
    
    # We simulate "less opacity" by using a very dark backdrop for the dialog container
    return Style.from_dict({
        "dialog": f"bg:{bg} #ffffff",
        "dialog.body": f"bg:{bg} #ffffff",
        "dialog.shadow": "bg:#080808",
        "dialog.border": accent,
        "dialog.title": f"bold {accent}",
        
        # Hover/Focus effect for buttons: reverse video + bold
        "button": f"bg:{bg} {accent}",
        "button.focused": f"bg:{accent} #000000 bold",
        "button.arrow": accent,
        
        # Hover/Focus effect for radiolist
        "radiolist": f"bg:{bg} #ffffff",
        "radiolist.radio": accent,
        "radiolist.radio.focused": f"bg:{accent} #000000 bold",
        "radiolist.item.focused": f"bg:{accent} #000000 bold",
        
        "input-field": "bg:#000000 #ffffff",
        "input-field.focused": f"bg:#000000 #ffffff border:{accent}",
        "label": "#ffffff",
        
        # This styles the area OUTSIDE the dialog (the "backdrop")
        "dialog-frame.label": f"bg:#111111 {accent}",
        "background": "bg:#0d0d0d", 
    })

def get_dialog_text(main_text: str, dialog_type: str = "radio") -> str:
    """Appends a navigation guide to the dialog text."""
    guides = {
        "radio": "\n\n[ Tab ] Focus Buttons  [ ↑↓ ] Select  [ Enter ] Confirm",
        "button": "\n\n[ Tab ] Switch Buttons  [ Enter ] Select",
        "input": "\n\n[ Tab ] Focus Buttons  [ Enter ] Submit",
        "message": "\n\n[ Enter ] OK",
    }
    guide = guides.get(dialog_type, guides["radio"])
    return f"{main_text}{guide}"

loom_theme = Theme(THEMES["lava"])
# The main console for user output
console = Console(theme=loom_theme, style=f"on {THEME_BACKGROUNDS['lava']}")

# The mirror console for backdrop capture
_mirror_output = io.StringIO()
mirror_console = Console(theme=loom_theme, file=_mirror_output, force_terminal=True, width=shutil.get_terminal_size().columns)

def _get_mirror_ansi() -> str:
    """Returns the current state of the terminal from the mirror."""
    return _mirror_output.getvalue()

# Patch the main console to also print to the mirror
_orig_print = console.print
def _mirrored_print(*args, **kwargs):
    _orig_print(*args, **kwargs)
    mirror_console.print(*args, **kwargs)
console.print = _mirrored_print

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


def print_welcome_screen(user_name: str, model: str, provider: str, target_console: Optional[Console] = None):
    c = target_console or console
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
    c.print(layout, height=max(15, h - 10))
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


def print_session_stats(state):
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
