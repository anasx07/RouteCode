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

from .base import _get_backdrop_ansi, BaseModalLayer
from .widgets import HoverRadioList, FlatButton
from ..terminal import TerminalManager
from ..theme import get_dialog_style


class LoomDialog(BaseModalLayer):
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
        super().__init__()
        self.title = title
        self.text = text
        self.dialog_type = dialog_type
        self.values = values or []
        self.buttons_config = buttons or [("OK", "ok"), ("Back", "cancel")]
        self.default = default
        self.password = password
        self.result = None
        self._focus_target = None

    def _get_focus_target(self):
        return self._focus_target

    def _build_container(self):
        kb = KeyBindings()

        @kb.add("c-c")
        @kb.add("escape", eager=True)
        def _(event):
            if not self.future.done():
                self.future.set_result(None)

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
                res = self._handle_result()
                if not self.future.done():
                    self.future.set_result(res)

            buttons.append(FlatButton(label, handler=handler))

        content = None
        if self.dialog_type == "radio":
            self.radio_list = HoverRadioList(self.values)
            self._focus_target = self.radio_list

            def _on_radio_enter():
                self.result = "ok"
                res = self._handle_result()
                if not self.future.done():
                    self.future.set_result(res)

            self.radio_list._on_enter = _on_radio_enter
            content = HSplit([Label(self.text), self.radio_list])
        elif self.dialog_type == "input":
            self.text_area = TextArea(
                text=self.default, password=self.password, multiline=False
            )
            self._focus_target = self.text_area
            content = HSplit([Label(self.text), self.text_area])

            @kb.add("enter")
            def _(event):
                self.result = "ok"
                res = self._handle_result()
                if not self.future.done():
                    self.future.set_result(res)
        elif self.dialog_type == "button":
            content = Label(self.text)
            if buttons:
                self._focus_target = buttons[0]
        elif self.dialog_type == "message":
            content = Label(self.text)
            def _ok_handler():
                if not self.future.done():
                    self.future.set_result(None)
            buttons = [FlatButton("OK", handler=_ok_handler)]
            self._focus_target = buttons[0]

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
        return Shadow(body=dialog_box)

    def _handle_result(self):
        if self.dialog_type == "radio" and self.result == "ok":
            return self.radio_list.current_value
        if self.dialog_type == "input" and self.result == "ok":
            return self.text_area.text
        if self.dialog_type in ("radio", "input"):
            return None
        return self.result
