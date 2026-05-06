from typing import Any, List, Dict, Optional
from prompt_toolkit.widgets import RadioList, Button, Dialog, Label, TextArea
from prompt_toolkit.layout.containers import Window, HSplit, VSplit, FloatContainer, Float, WindowAlign
from prompt_toolkit.layout.controls import FormattedTextControl
from prompt_toolkit.layout.layout import Layout as PTLayout
from prompt_toolkit.layout.menus import CompletionsMenu, CompletionsMenuControl
from prompt_toolkit.layout import ScrollOffsets
from prompt_toolkit.application import Application, get_app
from prompt_toolkit.key_binding import KeyBindings
from prompt_toolkit.formatted_text import ANSI
from prompt_toolkit.mouse_events import MouseEvent, MouseEventType
from .terminal import TerminalManager
from .theme import get_theme_bg, get_dialog_style
from .console import _mirror_output

class HoverCompletionsMenuControl(CompletionsMenuControl):
    """CompletionsMenuControl with MOUSE_MOVE hover support."""
    def mouse_handler(self, mouse_event: MouseEvent):
        b = get_app().current_buffer
        if mouse_event.event_type == MouseEventType.MOUSE_MOVE:
            b.go_to_completion(mouse_event.position.y)
        elif mouse_event.event_type == MouseEventType.MOUSE_UP:
            b.go_to_completion(mouse_event.position.y)
            b.complete_state = None
        elif mouse_event.event_type == MouseEventType.SCROLL_DOWN:
            b.complete_next(count=3, disable_wrap_around=True)
        elif mouse_event.event_type == MouseEventType.SCROLL_UP:
            b.complete_previous(count=3, disable_wrap_around=True)
        return None

class HoverCompletionsMenu(CompletionsMenu):
    def __init__(self, max_height=None, scroll_offset=0, extra_filter=True, 
                 display_arrows=False, z_index=10**8):
        from prompt_toolkit.filters import to_filter, has_completions, is_done
        from prompt_toolkit.layout.dimension import Dimension
        from prompt_toolkit.layout.margins import ScrollbarMargin
        
        extra_filter = to_filter(extra_filter)
        display_arrows = to_filter(display_arrows)
        
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
    def __init__(self, values):
        super().__init__(values)
        self.window.scroll_offsets = ScrollOffsets(top=3, bottom=3)
    
    def _get_text_fragments(self):
        from prompt_toolkit.formatted_text import to_formatted_text
        
        def mouse_handler(mouse_event: MouseEvent) -> None:
            if mouse_event.event_type == MouseEventType.MOUSE_MOVE:
                self._selected_index = mouse_event.position.y
            elif mouse_event.event_type == MouseEventType.MOUSE_UP:
                self._selected_index = mouse_event.position.y
                self._handle_enter()
            elif mouse_event.event_type == MouseEventType.SCROLL_UP:
                self._selected_index = max(0, self._selected_index - 3)
                get_app().invalidate()
            elif mouse_event.event_type == MouseEventType.SCROLL_DOWN:
                self._selected_index = min(len(self.values) - 1, self._selected_index + 3)
                get_app().invalidate()

        result = []
        for i, value in enumerate(self.values):
            checked = value[0] == self.current_value
            selected = i == self._selected_index
            style = ""
            if checked: style += " " + self.checked_style
            if selected: style += " " + self.selected_style
            result.append((style, self.open_character))
            if selected: result.append(("[SetCursorPosition]", ""))
            if checked: result.append((style, self.select_character))
            else: result.append((style, " "))
            result.append((style, self.close_character))
            result.append((f"{style} {self.default_style}", " "))
            result.extend(to_formatted_text(value[1], style=f"{style} {self.default_style}"))
            result.append(("", "\n"))

        for i in range(len(result)):
            result[i] = (result[i][0], result[i][1], mouse_handler)

        if result: result.pop()
        return result

class LoomDialog:
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
        full_ansi = _mirror_output.getvalue()
        from prompt_toolkit.output.defaults import create_output
        h = create_output().get_size().rows
        lines = full_ansi.splitlines()
        recent_lines = lines[-h:]
        ansi_content = "\n".join(recent_lines)
        return f"\033[2m{ansi_content}\033[0m"

    async def run_async(self):
        backdrop_ansi = self._get_backdrop_ansi()
        kb = KeyBindings()
        @kb.add("c-c")
        @kb.add("escape")
        def _(event): event.app.exit()

        buttons = []
        for label, value in self.buttons_config:
            def handler(v=value):
                self.result = v
                app.exit()
            buttons.append(Button(label, handler=handler))

        content = None
        if self.dialog_type == "radio":
            self.radio_list = HoverRadioList(self.values)
            content = HSplit([Label(self.text), self.radio_list])
        elif self.dialog_type == "input":
            self.text_area = TextArea(text=self.default, password=self.password, multiline=False)
            content = HSplit([Label(self.text), self.text_area])
            @kb.add("enter")
            def _(event):
                self.result = "ok"
                app.exit()
        elif self.dialog_type == "button":
            content = Label(self.text)
        elif self.dialog_type == "message":
            content = Label(self.text)
            buttons = [Button("OK", handler=lambda: app.exit())]

        dialog = Dialog(title=self.title, body=content, buttons=buttons, with_background=True)
        root_container = FloatContainer(
            content=Window(content=FormattedTextControl(ANSI(backdrop_ansi)), align=WindowAlign.LEFT),
            floats=[Float(content=dialog)]
        )
        TerminalManager.enable_mouse_tracking()
        app = Application(layout=PTLayout(root_container), key_bindings=kb, style=get_dialog_style(), full_screen=True, mouse_support=True)
        try:
            await app.run_async()
        finally:
            TerminalManager.disable_mouse_tracking()
        return self._handle_result()

    def _handle_result(self):
        if self.dialog_type == "radio" and self.result == "ok":
            return self.radio_list.current_value
        if self.dialog_type == "input" and self.result == "ok":
            return self.text_area.text
        if self.dialog_type in ("radio", "input"):
            return None
        return self.result

def get_dialog_text(main_text: str, dialog_type: str = "radio") -> str:
    guides = {
        "radio": "\n\n[ Tab ] Focus Buttons  [ ↑↓ ] Select  [ Enter ] Confirm",
        "button": "\n\n[ Tab ] Switch Buttons  [ Enter ] Select",
        "input": "\n\n[ Tab ] Focus Buttons  [ Enter ] Submit",
        "message": "\n\n[ Enter ] OK",
    }
    guide = guides.get(dialog_type, guides["radio"])
    return f"{main_text}{guide}"

class MenuRadioList(RadioList):
    def __init__(self, values, active_value=None, on_hover=None):
        self.on_hover = on_hover
        super().__init__(values)
        self.active_value = active_value
        self.window.scroll_offsets = ScrollOffsets(top=3, bottom=3)
        self.window.right_margins = []

    @property
    def _selected_index(self):
        return getattr(self, "__selected_index", 0)

    @_selected_index.setter
    def _selected_index(self, value):
        if not hasattr(self, 'values') or not self.values:
            self.__selected_index = 0
            return
            
        value = max(0, min(len(self.values) - 1, value))
        old_val = getattr(self, "__selected_index", None)
        self.__selected_index = value
        
        if old_val != value:
            if old_val is not None and getattr(self, "on_hover", None):
                try:
                    self.on_hover(self.values[value][0])
                except Exception:
                    pass
            from prompt_toolkit.application import get_app
            app = get_app()
            if app:
                app.invalidate()

    
    def _get_text_fragments(self):
        from prompt_toolkit.formatted_text import to_formatted_text
        import shutil
        
        def mouse_handler(mouse_event: MouseEvent) -> None:
            if mouse_event.event_type == MouseEventType.MOUSE_MOVE:
                self._selected_index = mouse_event.position.y
            elif mouse_event.event_type == MouseEventType.MOUSE_UP:
                self._selected_index = mouse_event.position.y
                self._handle_enter()
            elif mouse_event.event_type == MouseEventType.SCROLL_UP:
                self._selected_index -= 1
                get_app().invalidate()
            elif mouse_event.event_type == MouseEventType.SCROLL_DOWN:
                self._selected_index += 1
                get_app().invalidate()

        result = []
        # Fixed width for the menu highlight
        menu_width = 40

        for i, value in enumerate(self.values):
            is_active = value[0] == self.active_value
            selected = i == self._selected_index
            
            style = "class:menu-item"
            if selected:
                style += ".focused"
            elif is_active:
                style += ".active"
            
            prefix = "• " if is_active else "  "
            text_str = value[1]
            if isinstance(text_str, list):
                text_content = "".join(f[1] for f in text_str)
            else:
                text_content = str(text_str)
            
            display_text = f"{prefix}{text_content}".ljust(menu_width)
            
            result.extend(to_formatted_text(display_text, style=style))
            result.append(("", "\n"))

        for i in range(len(result)):
            result[i] = (result[i][0], result[i][1], mouse_handler)

        if result: result.pop()
        return result

class CategoryRadioList:
    """A lightweight list widget with non-selectable category headers and right-aligned tags."""
    def __init__(self, values, active_value=None, on_hover=None):
        self.on_hover = on_hover
        self.values = values
        self.active_value = active_value
        self._current_index = 0
        
        # Build a lightweight Window + FormattedTextControl (no RadioList overhead)
        self.control = FormattedTextControl(self._get_text_fragments, focusable=False)
        self.window = Window(
            content=self.control,
            scroll_offsets=ScrollOffsets(top=3, bottom=3),
            right_margins=[],
        )
        
        self._set_initial_index()

    def __pt_container__(self):
        return self.window

    @property
    def current_value(self):
        if self.values and 0 <= self._current_index < len(self.values):
            return self.values[self._current_index][0]
        return None

    def _set_initial_index(self):
        if not self.values: return
        if self.active_value:
            for i, v in enumerate(self.values):
                if v[0] == self.active_value and not v[2]:
                    self._current_index = i
                    return
        for i, v in enumerate(self.values):
            if not v[2]:
                self._current_index = i
                break

    @property
    def _selected_index(self):
        return self._current_index

    @_selected_index.setter
    def _selected_index(self, value):
        if not self.values:
            self._current_index = 0
            return
            
        old_val = self._current_index
        direction = 1 if value >= old_val else -1
        
        # Wrap around using modulo
        value = value % len(self.values)
        
        # Keep stepping in the same direction until we find a non-header
        attempts = 0
        while attempts < len(self.values):
            if len(self.values[value]) <= 2 or not self.values[value][2]:
                self._current_index = value
                return
            value = (value + direction) % len(self.values)
            attempts += 1
            
        # Fallback if no selectable items exist
        self._current_index = old_val

    def _get_text_fragments(self):
        def mouse_handler(mouse_event: MouseEvent) -> None:
            if mouse_event.event_type == MouseEventType.MOUSE_MOVE:
                self._selected_index = mouse_event.position.y
            elif mouse_event.event_type == MouseEventType.MOUSE_UP:
                idx = mouse_event.position.y
                if idx < len(self.values) and not self.values[idx][2]:
                    self._current_index = idx
                    # Signal enter via a callback if set
                    if hasattr(self, '_on_enter'):
                        self._on_enter()
            elif mouse_event.event_type == MouseEventType.SCROLL_UP:
                self._selected_index -= 1
            elif mouse_event.event_type == MouseEventType.SCROLL_DOWN:
                self._selected_index += 1

        result = []
        menu_width = 58

        for i, (value, label, is_header, description, tag) in enumerate(self.values):
            if is_header:
                result.append(("class:menu-header", f"\n{label}\n", mouse_handler))
                continue

            selected = i == self._current_index
            is_active = value == self.active_value
            
            if selected: 
                style = "class:menu-item-focused"
                prefix = "> "
            elif is_active: 
                style = "class:menu-item-active"
                prefix = "• "
            else:
                style = "class:menu-item"
                prefix = "  "
            
            main_text = f"{prefix}{label}"
            desc_text = f" {description}" if description else ""
            tag_text = str(tag) if tag else ""
            
            padding_len = max(1, menu_width - len(main_text) - len(desc_text) - len(tag_text))
            
            if selected:
                result.append(("[SetCursorPosition]", ""))
            result.append((style, main_text, mouse_handler))
            if description:
                result.append((style + "-dim", desc_text, mouse_handler))
            result.append((style, " " * padding_len, mouse_handler))
            if tag:
                result.append((style + "-tag", tag_text, mouse_handler))
            result.append(("", "\n", mouse_handler))

        if result: result.pop()
        return result

class PaletteMenu:
    def __init__(self, title: str, values: List[tuple], active_value: Optional[str] = None, on_hover=None):
        self.title = title
        self.values = values
        self.active_value = active_value
        self.on_hover = on_hover
        self.result = None

    def _get_backdrop_ansi(self) -> str:
        full_ansi = _mirror_output.getvalue()
        from prompt_toolkit.output.defaults import create_output
        try:
            h = create_output().get_size().rows
        except:
            import shutil
            h = shutil.get_terminal_size().lines
        lines = full_ansi.splitlines()
        recent_lines = lines[-h:]
        ansi_content = "\n".join(recent_lines)
        # Strong dimming for the backdrop
        return f"\033[2m\033[38;5;238m{ansi_content}\033[0m"

    async def run_async(self):
        backdrop_ansi = self._get_backdrop_ansi()
        kb = KeyBindings()
        
        @kb.add("c-c")
        @kb.add("escape")
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
            
        @kb.add("down", eager=True)
        def _(event):
            menu_list._selected_index += 1
            
        search_field = TextArea(multiline=False, prompt="search ", style="class:search")
        
        def on_text_changed(buf):
            query = buf.text.lower()
            if query:
                menu_list.values = [v for v in self.values if query in str(v[1]).lower()]
            else:
                menu_list.values = self.values
            menu_list._selected_index = 0
            
        search_field.buffer.on_text_changed += on_text_changed
        
        from prompt_toolkit.layout.dimension import D
        from prompt_toolkit.widgets import Box, Shadow

        header = VSplit([
            Window(FormattedTextControl(self.title), style="class:title"),
            Window(FormattedTextControl("esc"), align=WindowAlign.RIGHT, style="class:esc")
        ])

        menu_height = min(20, len(self.values) + 5)
        inner_split = HSplit([
            header,
            Window(height=1),
            search_field,
            Window(height=1),
            menu_list
        ], width=D(preferred=40, max=50), height=menu_height)
        
        menu_box = Box(
            body=inner_split,
            padding=1,
            style="class:container"
        )
        
        menu_container = Shadow(body=menu_box)

        dialog = FloatContainer(
            content=Window(content=FormattedTextControl(ANSI(backdrop_ansi)), align=WindowAlign.LEFT),
            floats=[Float(content=menu_container)]
        )

        TerminalManager.enable_mouse_tracking()
        from prompt_toolkit.styles import DynamicStyle
        app = Application(layout=PTLayout(dialog), key_bindings=kb, style=DynamicStyle(self._get_style), full_screen=True, mouse_support=True)
        try:
            await app.run_async()
        finally:
            TerminalManager.disable_mouse_tracking()
        return self.result

    def _get_style(self):
        from .theme import THEMES, _current_theme_name
        from prompt_toolkit.styles import Style
        
        theme = THEMES.get(_current_theme_name, THEMES["lava"])
        accent = theme.get("accent", "#ffaf00")
        
        return Style.from_dict({
            "title": "bold #ffffff",
            "esc": "dim #888888",
            "search": "fg:#888888 bg:#050505",
            "menu-item": "#888888",
            "menu-item.active": f"bold {accent}",
            "menu-item.focused": f"bg:{accent} #000000 bold",
            "menu-item.dim": "dim #666666",
            "menu-item.tag": "dim #888888",
            "menu-header": "bold #875fd7", # Purple header
            "container": "bg:#050505",
            "prompt": "dim #888888",
        })

class ModelPaletteMenu(PaletteMenu):
    def __init__(self, title: str, values: List[tuple], active_value: Optional[str] = None):
        # values: List[tuple] where each tuple is (id, name, is_header, description, tag)
        super().__init__(title, values, active_value)
        self.on_connect_provider = None
        self.on_favorite = None

    async def run_async(self):
        backdrop_ansi = self._get_backdrop_ansi()
        kb = KeyBindings()
        
        @kb.add("c-c")
        @kb.add("escape")
        def _(event): event.app.exit()

        menu_list = CategoryRadioList(self.values, self.active_value)
        
        # Wire up mouse click handler
        def _on_click_enter():
            if menu_list.values:
                self.result = menu_list.current_value
                get_app().exit()
        menu_list._on_enter = _on_click_enter
        
        search_field = TextArea(multiline=False, prompt="Search ", style="class:search")
        
        @kb.add("up", eager=True)
        def _(event):
            menu_list._selected_index -= 1
            
        @kb.add("down", eager=True)
        def _(event):
            menu_list._selected_index += 1

        @kb.add("enter", eager=True)
        def _(event):
            if menu_list.values:
                self.result = menu_list.current_value
                event.app.exit()

        @kb.add("c-a")
        def _(event):
            if self.on_connect_provider:
                self.result = "__connect_provider__"
                event.app.exit()

        @kb.add("c-f")
        def _(event):
            if self.on_favorite:
                # Toggle favorite for selected item
                current_item = menu_list.values[menu_list._selected_index]
                self.on_favorite(current_item[0])
                event.app.invalidate()
            
        def on_text_changed(buf):
            query = buf.text.lower()
            if query:
                # Filter items but keep headers if they have matching items? 
                # Or just filter all and hide empty headers.
                filtered = []
                current_header = None
                header_has_match = False
                
                for v in self.values:
                    if v[2]: # Header
                        if current_header and header_has_match:
                            pass # Already added previous header
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
            from prompt_toolkit.application import get_app
            app = get_app()
            if app: app.invalidate()
            
        search_field.buffer.on_text_changed += on_text_changed
        
        from prompt_toolkit.layout.dimension import D
        from prompt_toolkit.widgets import Box, Shadow

        header = VSplit([
            Window(FormattedTextControl(self.title), style="class:title"),
            Window(FormattedTextControl("esc"), align=WindowAlign.RIGHT, style="class:esc")
        ])

        footer = VSplit([
            Window(FormattedTextControl([
                ("class:footer-label", "Connect provider "),
                ("class:footer-key", "ctrl+a"),
                ("", "  "),
                ("class:footer-label", "Favorite "),
                ("class:footer-key", "ctrl+f"),
            ]), style="class:footer")
        ])

        menu_height = min(25, len(self.values) + 8)
        inner_split = HSplit([
            header,
            Window(height=1),
            search_field,
            Window(height=1),
            menu_list,
            Window(height=1),
            footer
        ], width=D(preferred=60, max=70), height=menu_height)
        
        menu_box = Box(
            body=inner_split,
            padding=1,
            style="class:container"
        )
        
        menu_container = Shadow(body=menu_box)

        TerminalManager.enable_mouse_tracking()
        from prompt_toolkit.formatted_text import to_formatted_text
        
        # Pre-parse the ANSI backdrop once
        backdrop_fragments = to_formatted_text(ANSI(backdrop_ansi))
        
        dialog = FloatContainer(
            content=Window(content=FormattedTextControl(backdrop_fragments), align=WindowAlign.LEFT),
            floats=[Float(content=menu_container)]
        )
        
        app = Application(
            layout=PTLayout(dialog, focused_element=search_field), 
            key_bindings=kb, 
            style=self._get_style(), 
            full_screen=True, 
            mouse_support=True
        )
        try:
            await app.run_async()
        finally:
            TerminalManager.disable_mouse_tracking()
        return self.result

    def _get_style(self):
        from .theme import THEMES, _current_theme_name
        from prompt_toolkit.styles import Style
        
        theme = THEMES.get(_current_theme_name, THEMES["lava"])
        accent = theme.get("accent", "#ffaf00")
        
        return Style.from_dict({
            "title": "bold #ffffff",
            "esc": "dim #888888",
            "search": "fg:#888888 bg:#050505",
            "menu-item": "#888888",
            "menu-item-focused": f"bg:{accent} #ffffff bold",
            "menu-item-focused-dim": f"bg:{accent} #cccccc",
            "menu-item-focused-tag": f"bg:{accent} #ffffff bold",
            "menu-item-active": "bg:#333333 #ffffff bold",
            "menu-item-active-dim": "bg:#222222 dim #666666",
            "menu-item-active-tag": "bg:#222222 dim #888888",
            "menu-item-dim": "dim #666666",
            "menu-item-tag": "dim #888888",
            "menu-header": "bold #875fd7", # Purple header
            "container": "bg:#050505",
            "prompt": "dim #888888",
            "footer": "#ffffff",
            "footer-label": "bold #ffffff",
            "footer-key": "dim #888888",
        })
