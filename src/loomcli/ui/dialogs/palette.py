from typing import List, Optional
from prompt_toolkit.widgets import Box, Shadow, TextArea
from prompt_toolkit.layout.containers import Window, HSplit, VSplit, FloatContainer, Float, WindowAlign
from prompt_toolkit.layout.controls import FormattedTextControl
from prompt_toolkit.layout.layout import Layout as PTLayout
from prompt_toolkit.application import Application, get_app
from prompt_toolkit.key_binding import KeyBindings
from prompt_toolkit.formatted_text import ANSI
from prompt_toolkit.layout.dimension import D
from prompt_toolkit.styles import DynamicStyle

from .base import _get_backdrop_ansi
from .widgets import MenuRadioList, CategoryRadioList
from ..terminal import TerminalManager
from ..theme import THEMES, THEME_ACCENTS, _current_theme_name, get_theme_bg

class PaletteMenu:
    def __init__(self, title: str, values: List[tuple], active_value: Optional[str] = None, on_hover=None):
        self.title = title
        self.values = values
        self.active_value = active_value
        self.on_hover = on_hover
        self.result = None

    async def run_async(self):
        backdrop_ansi = _get_backdrop_ansi()
        kb = KeyBindings()
        
        @kb.add("c-c")
        @kb.add("escape", eager=True)
        def _(event): event.app.exit()

        menu_list = MenuRadioList(self.values, self.active_value, self.on_hover)
        
        @kb.add("enter")
        def _(event):
            if menu_list.values:
                self.result = menu_list.current_value
                event.app.exit()
                
        @kb.add("up", eager=True)
        def _(event):
            menu_list._selected_index -= 1
            event.app.invalidate()
            
        @kb.add("down", eager=True)
        def _(event):
            menu_list._selected_index += 1
            event.app.invalidate()
            
        search_field = TextArea(multiline=False, prompt="search ", style="class:search")
        
        def on_text_changed(buf):
            query = buf.text.lower()
            if query:
                menu_list.values = [v for v in self.values if query in str(v[1]).lower()]
            else:
                menu_list.values = self.values
            menu_list._selected_index = 0
            
        search_field.buffer.on_text_changed += on_text_changed

        header = VSplit([
            Window(FormattedTextControl(self.title), style="class:title"),
            Window(FormattedTextControl("esc"), align=WindowAlign.RIGHT, style="class:esc")
        ])

        menu_height = min(24, len(self.values) + 8)
        
        footer = FormattedTextControl([
            ("class:footer-key", "[ ↑/↓ ] "), ("class:footer-label", "Navigate Items"),
            ("", "\n"),
            ("class:footer-key", "[ Enter ] "), ("class:footer-label", "Select"),
            ("", "   "),
            ("class:footer-key", "[ Esc ] "), ("class:footer-label", "Cancel"),
        ])

        inner_split = HSplit([
            header,
            Window(height=1),
            search_field,
            Window(height=1),
            menu_list,
            Window(height=1),
            Window(footer, height=2, style="class:footer")
        ], width=D(preferred=40, max=50), height=menu_height)
        
        menu_box = Box(
            body=inner_split,
            padding=1,
            style="class:container"
        )
        
        menu_container = Shadow(body=menu_box)

        # When on_hover is set (e.g. theme preview), intelligently swap accent colors
        # in the backdrop and strip background colors.
        if self.on_hover:
            import re
            def hex_to_rgb(h):
                h = h.lstrip('#')
                return tuple(int(h[i:i+2], 16) for i in (0, 2, 4))
            
            accent_rgbs = {hex_to_rgb(v): k for k, v in THEME_ACCENTS.items()}
            
            def get_preview_text():
                from ..theme import _current_theme_name
                target_hex = THEME_ACCENTS.get(_current_theme_name, "#ff0000")
                tr, tg, tb = hex_to_rgb(target_hex)
                processed = re.sub(r'\033\[(?:4[0-7]|10[0-7]|48;[25];[^m]*)m', '', backdrop_ansi)
                def replace_truecolor(match):
                    r, g, b = int(match.group(1)), int(match.group(2)), int(match.group(3))
                    if (r, g, b) in accent_rgbs:
                        return f'\033[38;2;{tr};{tg};{tb}m'
                    return match.group(0)
                processed = re.sub(r'\033\[38;2;(\d+);(\d+);(\d+)m', replace_truecolor, processed)
                return ANSI(processed)

            backdrop_content = Window(content=FormattedTextControl(get_preview_text), align=WindowAlign.LEFT)
        else:
            backdrop_content = Window(content=FormattedTextControl(ANSI(backdrop_ansi)), align=WindowAlign.LEFT)

        dialog = FloatContainer(
            content=backdrop_content,
            floats=[Float(content=menu_container)]
        )

        TerminalManager.enable_mouse_tracking()
        app = Application(
            layout=PTLayout(dialog, focused_element=search_field), 
            key_bindings=kb, 
            style=DynamicStyle(self._get_style), 
            full_screen=True, 
            mouse_support=True
        )
        try:
            await app.run_async()
        finally:
            TerminalManager.disable_mouse_tracking()
        return self.result

    def _get_style(self):
        from ..theme import THEMES, _current_theme_name, get_theme_bg
        from prompt_toolkit.styles import Style
        
        theme = THEMES.get(_current_theme_name, THEMES["lava"])
        accent = theme.get("accent", "#ffaf00")
        bg = get_theme_bg()
        
        return Style.from_dict({
            "": f"bg:{bg}",
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
            "prompt": "dim #888888",
            "footer": "#ffffff",
            "footer-label": "bold #ffffff",
            "footer-key": "dim #888888",
        })

class ModelPaletteMenu(PaletteMenu):
    def __init__(self, title: str, values: List[tuple], active_value: Optional[str] = None):
        super().__init__(title, values, active_value)
        self.on_connect_provider = None
        self.on_favorite = None

    async def run_async(self):
        backdrop_ansi = _get_backdrop_ansi()
        kb = KeyBindings()
        
        @kb.add("c-c")
        @kb.add("escape", eager=True)
        def _(event): event.app.exit()

        menu_list = CategoryRadioList(self.values, self.active_value)
        
        def _on_click_enter():
            if menu_list.values:
                self.result = menu_list.current_value
                get_app().exit()
        menu_list._on_enter = _on_click_enter
        
        search_field = TextArea(multiline=False, prompt="Search ", style="class:search")
        
        def on_text_changed(buf):
            query = buf.text.lower()
            if query:
                filtered = []
                current_header = None
                header_has_match = False
                for v in self.values:
                    if v[2]: # Header
                        current_header = v
                        header_has_match = False
                    else:
                        if query in str(v[1]).lower() or query in str(v[3] or "").lower():
                            if current_header and not header_has_match:
                                filtered.append(current_header)
                                header_has_match = True
                            filtered.append(v)
                menu_list.values = filtered
            else:
                menu_list.values = self.values
            menu_list._selected_index = 0
            app = get_app()
            if app:
                app.invalidate()
            
        search_field.buffer.on_text_changed += on_text_changed

        @kb.add("up", eager=True)
        def _(event):
            menu_list._selected_index -= 1
            event.app.invalidate()
            
        @kb.add("down", eager=True)
        def _(event):
            menu_list._selected_index += 1
            event.app.invalidate()

        @kb.add("left")
        def _(event):
            curr = menu_list._current_index
            for i in range(curr - 1, -1, -1):
                if self.values[i][2]: # Found a header
                    menu_list._selected_index = i
                    event.app.invalidate()
                    return
            for i in range(len(self.values) - 1, curr, -1):
                if self.values[i][2]:
                    menu_list._selected_index = i
                    event.app.invalidate()
                    return

        @kb.add("right")
        def _(event):
            curr = menu_list._current_index
            for i in range(curr + 1, len(self.values)):
                if self.values[i][2]:
                    menu_list._selected_index = i
                    event.app.invalidate()
                    return
            for i in range(0, curr):
                if self.values[i][2]:
                    menu_list._selected_index = i
                    event.app.invalidate()
                    return

        @kb.add("c-a")
        def _(event):
            if self.on_connect_provider:
                val = menu_list.current_value
                self.on_connect_provider(val)

        @kb.add("c-f")
        def _(event):
            if self.on_favorite:
                val = menu_list.current_value
                new_state = self.on_favorite(val)
                for i, v in enumerate(self.values):
                    if v[0] == val:
                        label = v[1]
                        if new_state:
                            if not label.startswith("★ "):
                                label = "★ " + label.lstrip()
                        else:
                            if label.startswith("★ "):
                                label = "  " + label[2:]
                        self.values[i] = (v[0], label, v[2], v[3], v[4])
                
                new_idx = menu_list._selected_index
                for i, v in enumerate(menu_list.values):
                    if v[0] == val:
                        label = v[1]
                        if new_state:
                            if not label.startswith("★ "):
                                label = "★ " + label.lstrip()
                        else:
                            if label.startswith("★ "):
                                label = "  " + label[2:]
                        menu_list.values[i] = (v[0], label, v[2], v[3], v[4])
                        new_idx = i
                        break
                menu_list._selected_index = new_idx
                event.app.invalidate()

        @kb.add("enter")
        def _(event):
            if menu_list.values:
                self.result = menu_list.current_value
                event.app.exit()

        header = VSplit([
            Window(FormattedTextControl(self.title), style="class:title"),
            Window(FormattedTextControl("esc"), align=WindowAlign.RIGHT, style="class:esc")
        ])

        items = [
            header,
            Window(height=1),
            search_field,
            Window(height=1),
            menu_list,
        ]
        
        if getattr(self, "show_footer", True):
            footer = VSplit([
                Window(FormattedTextControl([
                    ("class:footer-key", "[ Ctrl+A ] "),
                    ("class:footer-label", "Connect Provider"),
                    ("", "    "),
                    ("class:footer-key", "[ Ctrl+F ] "),
                    ("class:footer-label", "Favorite Model"),
                    ("", "\n\n"),
                    ("class:footer-key", "[ ↑/↓ ] "),
                    ("class:footer-label", "Navigate Models"),
                    ("", "      "),
                    ("class:footer-key", "[ ←/→ ] "),
                    ("class:footer-label", "Jump Category"),
                ]), style="class:footer")
            ])
            items.extend([Window(height=1), footer])

        menu_height = min(26, len(self.values) + 10)
        inner_split = HSplit(items, width=D(preferred=60, max=70), height=menu_height)
        
        menu_box = Box(
            body=inner_split,
            padding=1,
            style="class:container"
        )
        
        menu_container = Shadow(body=menu_box)
        TerminalManager.enable_mouse_tracking()
        dialog = FloatContainer(
            content=Window(content=FormattedTextControl(ANSI(backdrop_ansi)), align=WindowAlign.LEFT),
            floats=[Float(content=menu_container)]
        )
        
        app = Application(
            layout=PTLayout(dialog, focused_element=search_field), 
            key_bindings=kb, 
            style=DynamicStyle(self._get_style), 
            full_screen=True, 
            mouse_support=True
        )
        try:
            await app.run_async()
        finally:
            TerminalManager.disable_mouse_tracking()
        return self.result

    def _get_style(self):
        theme = THEMES.get(_current_theme_name, THEMES["lava"])
        accent = theme.get("accent", "#ffaf00")
        bg = get_theme_bg()
        
        from prompt_toolkit.styles import Style
        return Style.from_dict({
            "": f"bg:{bg}",
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
            "prompt": "dim #888888",
            "footer": "#ffffff",
            "footer-label": "bold #ffffff",
            "footer-key": "dim #888888",
        })
