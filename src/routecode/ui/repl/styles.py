from prompt_toolkit.styles import Style
from prompt_toolkit.lexers import Lexer
from prompt_toolkit.formatted_text import ANSI
from prompt_toolkit.output.vt100 import Vt100_Output

from ..theme import get_theme_bg, get_theme_accent
from ...utils.helpers import parse_hex_color


def _get_active_bg(is_dimmed: bool = False) -> str:
    bg = get_theme_bg()
    if is_dimmed:
        try:
            r, g, b = parse_hex_color(bg)
            return f"#{int(r * 0.4):02x}{int(g * 0.4):02x}{int(b * 0.4):02x}"
        except Exception:
            pass
    return bg


class RouteCodeVt100Output(Vt100_Output):
    """
    Custom VT100 output that ensures the theme background color is preserved
    during screen clearing and attribute resets.
    """

    def __init__(self, stdout, get_size, state_provider=None, **kwargs):
        super().__init__(stdout, get_size, **kwargs)
        self.state_provider = state_provider or (lambda: False)

    def erase_end_of_line(self):
        bg = _get_active_bg(is_dimmed=self.state_provider())
        try:
            r, g, b = parse_hex_color(bg)
            self.write_raw(f"\033[48;2;{r};{g};{b}m")
        except Exception:
            pass
        super().erase_end_of_line()

    def reset_attributes(self):
        super().reset_attributes()
        bg = _get_active_bg(is_dimmed=self.state_provider())
        try:
            r, g, b = parse_hex_color(bg)
            self.write_raw(f"\033[48;2;{r};{g};{b}m")
        except Exception:
            pass


class SimpleAnsiLexer(Lexer):
    def __init__(self, state_provider=None):
        self.state_provider = state_provider or (lambda: False)

    def lex_document(self, document):
        is_dimmed = self.state_provider()

        def get_line(i):
            line = document.lines[i]
            ft = ANSI(line).__pt_formatted_text__()

            if is_dimmed:
                # Dynamically remap the parsed ANSI tuples to dimmed styles
                dimmed_ft = []
                for style, text in ft:
                    # Strip any explicit colors from the parsed style and apply a global dimming class
                    # We preserve non-color attributes like bold/italic if desired, or just force a dim class
                    dimmed_ft.append(("class:text-dimmed", text))
                return dimmed_ft

            return ft

        return get_line


def _is_modal_open() -> bool:
    try:
        from prompt_toolkit.application.current import get_app
        from prompt_toolkit.widgets import Shadow

        app = get_app()
        if app and app.is_running and hasattr(app.layout.container, "floats"):
            for f in app.layout.container.floats:
                if isinstance(f.content, Shadow):
                    return True
    except Exception:
        pass
    return False


def build_repl_style(is_dimmed: bool = False):
    bg = get_theme_bg()
    active_bg = _get_active_bg(is_dimmed)
    accent = get_theme_accent()

    if is_dimmed:
        sidebar_bg = "#08080a"
        text_fg = "#555566"
        header_fg = "#444455"
        border_fg = "#151520"
        footer_bg = active_bg
    else:
        sidebar_bg = "#111118"
        text_fg = "#888899"
        header_fg = accent
        border_fg = "#2a2a40"
        footer_bg = bg

    return Style.from_dict(
        {
            "": f"bg:{active_bg} #ffffff",
            "history": f"bg:{active_bg}",
            "input-area": f"bg:{active_bg}",
            "status-bar": "bg:#0e0e1c #555566",
            "status-bar.workspace": "fg:#e0e0ff bold",
            "status-bar.model": "fg:#ffaf00",
            "status-bar.sep": "fg:#333355",
            "status-bar.metrics": "fg:#555577",
            "prompt": f"fg:{header_fg} bold",
            "divider": f"fg:{border_fg}",
            "input-box": f"bg:{'#15151c' if is_dimmed else '#22222a'} #ffffff",
            "input-box-model": f"bg:{'#15151c' if is_dimmed else '#22222a'} fg:{text_fg}",
            # Welcome mode styles
            "welcome-logo": f"fg:{header_fg}",
            "welcome-input": f"bg:{bg} #ffffff",
            "welcome-model": f"bg:{bg}",
            "welcome-hint": f"bg:{bg} fg:#555566",
            "welcome-tip": f"bg:{bg} fg:#555566",
            # Session mode styles
            "sidebar": f"bg:{sidebar_bg} {text_fg}",
            "sidebar-border": f"fg:{border_fg}",
            "sidebar-title": f"bg:{sidebar_bg} {'#666677' if is_dimmed else '#ffffff'} bold",
            "sidebar-header": f"bg:{sidebar_bg} {header_fg} bold",
            "sidebar-value": f"bg:{sidebar_bg} {text_fg}",
            "sidebar-footer": f"bg:{sidebar_bg} {header_fg}",
            "session-footer": f"bg:{footer_bg} fg:#555566",
            "session-footer-key": f"bg:#1a1a33 {header_fg} bold",
            "session-footer-label": f"bg:{footer_bg} {'#666677' if is_dimmed else '#ccccdd'} bold",
            "user-message": f"fg:{'#888899' if is_dimmed else '#ffffff'} bold",
            "thought": f"fg:{'#333344' if is_dimmed else '#555577'} italic",
            "text-dimmed": "#444455",
            # Dialog styles (PaletteMenu)
            "title": "bold #ffffff",
            "esc": "dim #888888",
            "search": f"fg:#888888 bg:{bg}",
            "menu-item": "#888888",
            "menu-item-focused": f"bg:{accent} #ffffff bold",
            "menu-item-focused-dim": f"bg:{accent} #cccccc",
            "menu-item-focused-tag": f"bg:{accent} #ffffff bold",
            "menu-item-active": "bg:#333333 #ffffff bold",
            "menu-item-active-dim": "bg:#222222 dim #666666",
            "menu-item-active-tag": "bg:#222222 dim #888888",
            "menu-item-dim": "dim #666666",
            "menu-item-tag": "dim #888888",
            "menu-header": f"bold {accent}",
            "container": f"bg:{bg}",
            "footer": "#666677",
            "footer-label": "#ccccdd bold",
            "footer-key": f"bg:#1a1a33 {accent} bold",
            "footer-dim": "#333355",
        }
    )
