from prompt_toolkit.widgets import RadioList
from prompt_toolkit.layout.containers import Window
from prompt_toolkit.layout.controls import FormattedTextControl
from prompt_toolkit.layout.menus import CompletionsMenu, CompletionsMenuControl
from prompt_toolkit.layout import ScrollOffsets
from prompt_toolkit.application import get_app
from prompt_toolkit.key_binding import KeyBindings
from prompt_toolkit.mouse_events import MouseEvent, MouseEventType


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
    def __init__(
        self,
        max_height=None,
        scroll_offset=0,
        extra_filter=True,
        display_arrows=False,
        z_index=10**8,
    ):
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


class HoverRadioList:
    """A lightweight list widget for dialogs with hover and click support."""

    def __init__(self, values):
        self.values = values
        self._current_index = 0
        self._on_enter = None

        self.control = FormattedTextControl(self._get_text_fragments, focusable=True)
        self.window = Window(
            content=self.control,
            scroll_offsets=ScrollOffsets(top=3, bottom=3),
            right_margins=[],
        )

        kb = KeyBindings()

        @kb.add("up")
        def _(event):
            self._selected_index -= 1

        @kb.add("down")
        def _(event):
            self._selected_index += 1

        @kb.add("enter")
        def _(event):
            if self._on_enter:
                self._on_enter()

        self.control.key_bindings = kb

    def __pt_container__(self):
        return self.window

    @property
    def current_value(self):
        if self.values and 0 <= self._current_index < len(self.values):
            return self.values[self._current_index][0]
        return None

    @property
    def _selected_index(self):
        return self._current_index

    @_selected_index.setter
    def _selected_index(self, value):
        if not self.values:
            return
        self._current_index = value % len(self.values)

    def _get_text_fragments(self):
        def mouse_handler(mouse_event: MouseEvent) -> None:
            if mouse_event.event_type == MouseEventType.MOUSE_MOVE:
                self._selected_index = mouse_event.position.y
            elif mouse_event.event_type == MouseEventType.MOUSE_UP:
                idx = mouse_event.position.y
                if idx < len(self.values):
                    self._current_index = idx
                    if self._on_enter:
                        self._on_enter()
            elif mouse_event.event_type == MouseEventType.SCROLL_UP:
                self._selected_index -= 1
            elif mouse_event.event_type == MouseEventType.SCROLL_DOWN:
                self._selected_index += 1

        result = []
        menu_width = 40

        for i, (value, label) in enumerate(self.values):
            selected = i == self._current_index

            if selected:
                style = "class:menu-item-focused"
                prefix = "> "
            else:
                style = "class:menu-item"
                prefix = "  "

            main_text = f"{prefix}{label}"
            padding_len = max(1, menu_width - len(main_text))

            if selected:
                result.append(("[SetCursorPosition]", ""))
            result.append((style, main_text, mouse_handler))
            result.append((style, " " * padding_len, mouse_handler))
            result.append(("", "\n", mouse_handler))

        if result:
            result.pop()
        return result


class FlatButton:
    """A minimalist button that behaves like palette menu items."""

    def __init__(self, text: str, handler):
        self.text = text
        self.handler = handler

        kb = KeyBindings()

        @kb.add("enter")
        @kb.add(" ")
        def _(event):
            if self.handler:
                self.handler()

        self.control = FormattedTextControl(
            self._get_text_fragments,
            key_bindings=kb,
            focusable=True,
        )

        def get_style():
            if get_app().layout.has_focus(self.control):
                return "class:menu-item-focused"
            return "class:menu-item"

        self.window = Window(
            self.control,
            dont_extend_width=True,
            dont_extend_height=True,
            style=get_style,
        )

    def __pt_container__(self):
        return self.window

    def _get_text_fragments(self):
        is_focused = get_app().layout.has_focus(self.control)

        def mouse_handler(mouse_event):
            if mouse_event.event_type == MouseEventType.MOUSE_UP:
                if self.handler:
                    self.handler()
            elif mouse_event.event_type == MouseEventType.MOUSE_MOVE:
                get_app().layout.focus(self.control)

        if is_focused:
            return [("[SetCursorPosition]", ""), ("", f"> {self.text} ", mouse_handler)]
        return [("", f"  {self.text} ", mouse_handler)]


class MenuRadioList(RadioList):
    def __init__(self, values, active_value=None, on_hover=None):
        self.on_hover = on_hover
        super().__init__(values)
        self.active_value = active_value
        self.window.scroll_offsets = ScrollOffsets(top=3, bottom=3)
        self.window.right_margins = []

    @property
    def _selected_index(self):
        return getattr(self, "_current_index", 0)

    @_selected_index.setter
    def _selected_index(self, value):
        if not hasattr(self, "values") or not self.values:
            self._current_index = 0
            return

        value = max(0, min(len(self.values) - 1, value))
        old_val = getattr(self, "_current_index", None)
        self._current_index = value

        if old_val != value:
            app = get_app()
            if getattr(self, "on_hover", None):
                try:
                    self.on_hover(self.values[value][0])
                except Exception:
                    pass
            if app:
                app.invalidate()

    def _get_text_fragments(self):
        from prompt_toolkit.formatted_text import to_formatted_text

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
        menu_width = 40

        for i, value in enumerate(self.values):
            is_active = value[0] == self.active_value
            selected = i == self._selected_index

            if selected:
                style = "class:menu-item-focused"
                prefix = "> "
            elif is_active:
                style = "class:menu-item-active"
                prefix = "• "
            else:
                style = "class:menu-item"
                prefix = "  "

            text_str = value[1]
            if isinstance(text_str, list):
                text_content = "".join(f[1] for f in text_str)
            else:
                text_content = str(text_str)

            display_text = f"{prefix}{text_content}".ljust(menu_width)

            if selected:
                result.append(("[SetCursorPosition]", ""))

            result.extend(to_formatted_text(display_text, style=style))
            result.append(("", "\n"))

        for i in range(len(result)):
            result[i] = (result[i][0], result[i][1], mouse_handler)

        if result:
            result.pop()
        return result


class CategoryRadioList:
    """A lightweight list widget with non-selectable category headers and right-aligned tags."""

    def __init__(self, values, active_value=None, on_hover=None):
        self.on_hover = on_hover
        self.values = values
        self.active_value = active_value
        self._current_index = 0

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
        if not self.values:
            return
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
        value = value % len(self.values)

        attempts = 0
        while attempts < len(self.values):
            if len(self.values[value]) <= 2 or not self.values[value][2]:
                self._current_index = value
                return
            value = (value + direction) % len(self.values)
            attempts += 1
        self._current_index = old_val

    def _get_text_fragments(self):
        def mouse_handler(mouse_event: MouseEvent) -> None:
            if mouse_event.event_type == MouseEventType.MOUSE_MOVE:
                self._selected_index = mouse_event.position.y
            elif mouse_event.event_type == MouseEventType.MOUSE_UP:
                idx = mouse_event.position.y
                if idx < len(self.values) and not self.values[idx][2]:
                    self._current_index = idx
                    if hasattr(self, "_on_enter"):
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

            has_star = False
            if label.startswith("✓ "):
                check_text = "✓ "
                label_text = label[2:]
                has_check = True
            elif label.startswith("★ "):
                check_text = "★ "
                label_text = label[2:]
                has_check = False
                has_star = True
            elif label.startswith("  "):
                check_text = "  "
                label_text = label[2:]
                has_check = False
            else:
                check_text = ""
                label_text = label
                has_check = False

            main_text_raw = f"{prefix}{check_text}{label_text}"
            desc_text = f" {description}" if description else ""
            tag_text = str(tag) if tag else ""

            padding_len = max(
                1, menu_width - len(main_text_raw) - len(desc_text) - len(tag_text)
            )

            if selected:
                result.append(("[SetCursorPosition]", ""))

            result.append((style, prefix, mouse_handler))
            if has_check:
                check_style = f"{style} fg:ansigreen" if selected else "fg:ansigreen"
                result.append((check_style, check_text, mouse_handler))
            elif has_star:
                star_style = f"{style} fg:ansiyellow" if selected else "fg:ansiyellow"
                result.append((star_style, check_text, mouse_handler))
            else:
                result.append((style, check_text, mouse_handler))

            result.append((style, label_text, mouse_handler))
            if description:
                result.append((style + "-dim", desc_text, mouse_handler))
            result.append((style, " " * padding_len, mouse_handler))
            if tag:
                result.append((style + "-tag", tag_text, mouse_handler))
            result.append(("", "\n", mouse_handler))

        if result:
            result.pop()
        return result
