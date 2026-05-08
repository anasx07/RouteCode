from typing import List, Optional
from prompt_toolkit.widgets import Box, Shadow, TextArea
from prompt_toolkit.layout.containers import (
    Window,
    HSplit,
    VSplit,
    WindowAlign,
)
from prompt_toolkit.layout.controls import FormattedTextControl
from prompt_toolkit.application import get_app
from prompt_toolkit.key_binding import KeyBindings
from prompt_toolkit.layout.dimension import D

from .base import BaseModalLayer
from .widgets import MenuRadioList, CategoryRadioList


class PaletteMenu(BaseModalLayer):
    def __init__(
        self,
        title: str,
        values: List[tuple],
        active_value: Optional[str] = None,
        on_hover=None,
    ):
        super().__init__()
        self.title = title
        self.values = values
        self.active_value = active_value
        self.on_hover = on_hover
        self.result = None
        self._search_field = None

    def _get_focus_target(self):
        return self._search_field

    def _build_container(self):
        kb = KeyBindings()

        @kb.add("c-c")
        @kb.add("escape", eager=True)
        def _(event):
            if not self.future.done():
                self.future.set_result(None)

        menu_list = MenuRadioList(self.values, self.active_value, self.on_hover)

        @kb.add("enter")
        def _(event):
            if menu_list.values:
                self.result = menu_list.current_value
                if not self.future.done():
                    self.future.set_result(self.result)

        @kb.add("up", eager=True)
        def _(event):
            menu_list._selected_index -= 1
            event.app.invalidate()

        @kb.add("down", eager=True)
        def _(event):
            menu_list._selected_index += 1
            event.app.invalidate()

        self._search_field = TextArea(
            multiline=False, prompt="search ", style="class:search"
        )
        self._search_field.control.key_bindings = kb

        def on_text_changed(buf):
            query = buf.text.lower()
            if query:
                menu_list.values = [
                    v for v in self.values if query in str(v[1]).lower()
                ]
            else:
                menu_list.values = self.values
            menu_list._selected_index = 0

        self._search_field.buffer.on_text_changed += on_text_changed

        header = VSplit(
            [
                Window(FormattedTextControl(self.title), style="class:title"),
                Window(
                    FormattedTextControl("esc"),
                    align=WindowAlign.RIGHT,
                    style="class:esc",
                ),
            ]
        )

        menu_height = min(24, len(self.values) + 8)

        footer = FormattedTextControl(
            [
                ("class:footer-key", "[ ↑/↓ ] "),
                ("class:footer-label", "Navigate Items"),
                ("", "\n"),
                ("class:footer-key", "[ Enter ] "),
                ("class:footer-label", "Select"),
                ("", "   "),
                ("class:footer-key", "[ Esc ] "),
                ("class:footer-label", "Cancel"),
            ]
        )

        inner_split = HSplit(
            [
                header,
                Window(height=1),
                self._search_field,
                Window(height=1),
                menu_list,
                Window(height=1),
                Window(footer, height=2, style="class:footer"),
            ],
            width=D(preferred=40, max=50),
            height=menu_height,
        )

        menu_box = Box(body=inner_split, padding=1, style="class:container")
        return Shadow(body=menu_box)


class ModelPaletteMenu(PaletteMenu):
    def __init__(
        self, title: str, values: List[tuple], active_value: Optional[str] = None
    ):
        super().__init__(title, values, active_value)
        self.on_connect_provider = None
        self.on_favorite = None

    def _build_container(self):
        kb = KeyBindings()

        @kb.add("c-c")
        @kb.add("escape", eager=True)
        def _(event):
            if not self.future.done():
                self.future.set_result(None)

        menu_list = CategoryRadioList(self.values, self.active_value)

        def _on_click_enter():
            if menu_list.values:
                self.result = menu_list.current_value
                if not self.future.done():
                    self.future.set_result(self.result)

        menu_list._on_enter = _on_click_enter

        self._search_field = TextArea(
            multiline=False, prompt="/ ", style="class:search"
        )
        self._search_field.control.key_bindings = kb

        def on_text_changed(buf):
            query = buf.text.lower()
            if query:
                filtered = []
                current_header = None
                header_has_match = False
                for v in self.values:
                    if v[2]:  # Header
                        current_header = v
                        header_has_match = False
                    else:
                        if (
                            query in str(v[1]).lower()
                            or query in str(v[3] or "").lower()
                        ):
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

        self._search_field.buffer.on_text_changed += on_text_changed

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
                if self.values[i][2]:  # Found a header
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
                if not self.future.done():
                    self.future.set_result(self.result)

        header = VSplit(
            [
                Window(FormattedTextControl(self.title), style="class:title"),
                Window(
                    FormattedTextControl("esc"),
                    align=WindowAlign.RIGHT,
                    style="class:esc",
                ),
            ]
        )

        items = [
            header,
            Window(height=1),
            self._search_field,
            Window(height=1),
            menu_list,
        ]

        if getattr(self, "show_footer", True):
            footer = Window(
                FormattedTextControl(
                    [
                        ("class:footer-key", " Ctrl+A "),
                        ("class:footer-label", " Connect Provider"),
                        ("class:footer-dim", "   │   "),
                        ("class:footer-key", " Ctrl+F "),
                        ("class:footer-label", " Favorite\n"),
                        ("class:footer-key", " ↑/↓ "),
                        ("class:footer-label", " Navigate"),
                        ("class:footer-dim", "       │   "),
                        ("class:footer-key", " ←/→ "),
                        ("class:footer-label", " Jump Category"),
                    ]
                ),
                height=2,
                style="class:footer",
            )
            items.extend([Window(height=1), footer])

        menu_height = min(26, len(self.values) + 10)
        inner_split = HSplit(items, width=D(preferred=60, max=70), height=menu_height)

        menu_box = Box(body=inner_split, padding=1, style="class:container")
        return Shadow(body=menu_box)
