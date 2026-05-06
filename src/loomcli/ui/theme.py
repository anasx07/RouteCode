from rich.theme import Theme
from prompt_toolkit.styles import Style
from .terminal import TerminalManager

# Background color per theme (hex)
THEME_BACKGROUNDS = {
    "lava":     "#1a1a2e",
    "ocean":    "#0d1b2a",
    "forest":   "#0b1a0b",
    "sunset":   "#1f1008",
    "midnight": "#0a0a14",
}

BASE_THEME = {
    "info": "bright_black", "warning": "yellow", "error": "bold red",
    "success": "bold green", "prompt": "bold white", "command": "bold blue",
    "dim": "bright_black", "border": "bright_black",
    "title": "bold white", "toolbar": "white on grey15",
    "user": "bold white", "ai": "bold white", "thought": "italic bright_black",
    "tool_bash": "bold cyan", "tool_read": "bold blue",
    "tool_edit": "bold yellow", "tool_write": "bold magenta",
    "tool_glob": "bold green", "tool_grep": "bold green",
    "tool_task": "bold white", "tool_skill": "bold white",
    "tool_webfetch": "bold cyan", "stats_label": "bright_black",
    "stats_value": "white",
}

THEME_ACCENTS = {
    "lava": "#ff0000",
    "ocean": "#00afff",
    "forest": "#00d700",
    "sunset": "#ffaf00",
    "midnight": "#af87d7",
}

THEMES = {
    name: {
        **BASE_THEME, 
        "accent": accent, 
        "tool": accent,
        # Per-theme overrides
        **({"command": "bold cyan", "toolbar": "bright_black on grey3"} if name == "midnight" else {})
    }
    for name, accent in THEME_ACCENTS.items()
}

_current_theme_name = "lava"

def get_theme_bg(name=None):
    """Return the hex background color for the given (or current) theme."""
    return THEME_BACKGROUNDS.get(name or _current_theme_name, "#1a1a2e")

def apply_theme(name: str = "lava"):
    global _current_theme_name
    _current_theme_name = name
    theme_dict = THEMES.get(name, THEMES["lava"])
    loom_theme = Theme(theme_dict)

    bg = get_theme_bg(name)
    TerminalManager.set_background(bg)

    from .console import console, mirror_console, _mirror_output
    import shutil
    from rich.console import Console

    # Recreate the actual Rich console instances
    actual_console = Console(theme=loom_theme, force_terminal=True, color_system="truecolor", style=f"on {bg}")
    actual_mirror = Console(theme=loom_theme, file=_mirror_output, force_terminal=True, color_system="truecolor", style=f"on {bg}")
    
    # Patch the main console to also print to the mirror
    _orig_print = actual_console.print
    def _mirrored_print(*args, **kwargs):
        _orig_print(*args, **kwargs)
        actual_mirror.width = shutil.get_terminal_size().columns
        actual_mirror.print(*args, **kwargs)
    actual_console.print = _mirrored_print

    # Update the proxies
    console.set_instance(actual_console)
    mirror_console.set_instance(actual_mirror)

    # Notify listeners of the theme change
    from ..core import bus
    bus.emit("ui.theme_changed", name=name)

def get_dialog_style():
    """Returns a dynamic prompt_toolkit Style for interactive dialogs based on current theme."""
    bg = get_theme_bg()
    theme = THEMES.get(_current_theme_name, THEMES["lava"])
    accent = theme.get("accent", "#ffaf00")
    
    return Style.from_dict({
        "dialog": f"bg:{bg} #ffffff",
        "dialog.body": f"bg:{bg} #ffffff",
        "dialog.shadow": "bg:#080808",
        "dialog.border": accent,
        "dialog.title": f"bold {accent}",
        "button": f"bg:{bg} {accent}",
        "button.focused": f"bg:{accent} #000000 bold",
        "button.arrow": accent,
        "radiolist": f"bg:{bg} #ffffff",
        "radiolist.radio": accent,
        "radiolist.radio.focused": f"bg:{accent} #000000 bold",
        "radiolist.item.focused": f"bg:{accent} #000000 bold",
        "input-field": "bg:#000000 #ffffff",
        "input-field.focused": f"bg:#000000 #ffffff border:{accent}",
        "label": "#ffffff",
        "dialog-frame.label": f"bg:#111111 {accent}",
        "background": "bg:#0d0d0d", 
    })
