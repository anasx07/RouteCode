from typing import List, Optional
from prompt_toolkit.widgets import Label, TextArea, Box, Shadow
from prompt_toolkit.layout.containers import (
    Window,
    HSplit,
    VSplit,
    FloatContainer,
    Float,
    WindowAlign,
)
from prompt_toolkit.layout.controls import FormattedTextControl
from prompt_toolkit.layout.layout import Layout as PTLayout
from prompt_toolkit.application import Application
from prompt_toolkit.key_binding import KeyBindings
from prompt_toolkit.formatted_text import ANSI
from prompt_toolkit.layout.dimension import D
from prompt_toolkit.styles import DynamicStyle

from .base import _get_backdrop_ansi
from .widgets import HoverRadioList, FlatButton
from ..terminal import TerminalManager
from ..theme import get_dialog_style


class LoomDialog:
    def __init__(
        self,
        title: str,
        text: str,
        dialog_type: str = "radio",
        values: Optional[List[tuple]] = None,
        buttons: Optional[List[tuple]] = None,
        default: str = "",
        password: bool = False,
    ):
        self.title = title
        self.text = text
        self.dialog_type = dialog_type
        self.values = values or []
        self.buttons_config = buttons or [("OK", "ok"), ("Back", "cancel")]
        self.default = default
        self.password = password
        self.result = None

    async def run_async(self):
        backdrop_ansi = _get_backdrop_ansi()
        kb = KeyBindings()

        @kb.add("c-c")
        @kb.add("escape", eager=True)
        def _(event):
            event.app.exit()

        # Add intuitive arrow navigation for buttons
        @kb.add("right")
        def _(event):
            if self.dialog_type != "input":
                event.app.layout.focus_next()

        @kb.add("left")
        def _(event):
            if self.dialog_type != "input":
                event.app.layout.focus_previous()

        @kb.add("down")
        def _(event):
            event.app.layout.focus_next()

        @kb.add("up")
        def _(event):
            event.app.layout.focus_previous()

        buttons = []
        for label, value in self.buttons_config:

            def handler(v=value):
                self.result = v
                get_app().exit()

            buttons.append(FlatButton(label, handler=handler))

        from prompt_toolkit.application import get_app

        content = None
        if self.dialog_type == "radio":
            self.radio_list = HoverRadioList(self.values)

            def _on_radio_enter():
                self.result = "ok"
                get_app().exit()

            self.radio_list._on_enter = _on_radio_enter
            content = HSplit([Label(self.text), self.radio_list])
        elif self.dialog_type == "input":
            self.text_area = TextArea(
                text=self.default, password=self.password, multiline=False
            )
            content = HSplit([Label(self.text), self.text_area])

            @kb.add("enter")
            def _(event):
                self.result = "ok"
                event.app.exit()
        elif self.dialog_type == "button":
            content = Label(self.text)
        elif self.dialog_type == "message":
            content = Label(self.text)
            buttons = [FlatButton("OK", handler=lambda: get_app().exit())]

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

        # Centered buttons at the bottom
        if buttons:
            spacer = Window(width=D(weight=1))
            button_items = [spacer] + buttons + [spacer]
            button_split = VSplit(button_items, padding=2)
        else:
            button_split = None

        items = [
            header,
            Window(height=1),
            content,
        ]
        if button_split:
            items.extend([Window(height=1), button_split])

        # Dynamic height estimation
        dialog_height = 8
        if self.dialog_type == "radio":
            dialog_height += len(self.values)
        if self.dialog_type == "input":
            dialog_height += 2

        inner_split = HSplit(items, width=D(preferred=60, max=70), height=dialog_height)

        dialog_box = Box(body=inner_split, padding=1, style="class:container")
        dialog_container = Shadow(body=dialog_box)

        root_container = FloatContainer(
            content=Window(
                content=FormattedTextControl(ANSI(backdrop_ansi)),
                align=WindowAlign.LEFT,
            ),
            floats=[Float(content=dialog_container)],
        )
        TerminalManager.enable_mouse_tracking()
        app = Application(
            layout=PTLayout(root_container),
            key_bindings=kb,
            style=DynamicStyle(get_dialog_style),
            full_screen=True,
            mouse_support=True,
        )
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
