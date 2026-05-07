"""Patch ui.py to apply theme background to the entire terminal."""
import pathlib

ui_path = pathlib.Path(r"d:\DEV\Apps\Loom\src\loomcli\ui.py")
content = ui_path.read_text(encoding="utf-8", errors="replace")

old = '''def apply_theme(name: str = "lava"):
    theme_dict = THEMES.get(name, THEMES["lava"])
    global loom_theme
    loom_theme = Theme(theme_dict)
    console._theme_stack._entries.clear()
    console._theme_stack.push_theme(Theme({}), inherit=False)
    console._theme_stack.push_theme(loom_theme)


loom_theme = Theme(THEMES["lava"])
console = Console(theme=loom_theme)'''

new = '''_current_theme_name = "lava"


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
    sys.stdout.write(f"\\033]11;rgb:{r:02x}/{g:02x}/{b:02x}\\033\\\\")
    # SGR 48;2 sets background for subsequent text
    sys.stdout.write(f"\\033[48;2;{r};{g};{b}m")
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


loom_theme = Theme(THEMES["lava"])
console = Console(theme=loom_theme, style=f"on {THEME_BACKGROUNDS[\'lava\']}")'''

if old in content:
    content = content.replace(old, new)
    ui_path.write_text(content, encoding="utf-8")
    print("SUCCESS: ui.py patched")
else:
    print("ERROR: target block not found")
    idx = content.find("def apply_theme")
    if idx >= 0:
        print(f"Found at index {idx}")
        print(repr(content[idx:idx+300]))
    else:
        print("apply_theme not found at all!")
