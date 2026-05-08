import os
import time
from prompt_toolkit.layout import HSplit, VSplit, Window, WindowAlign, Dimension
from prompt_toolkit.layout.controls import BufferControl, FormattedTextControl
from prompt_toolkit.mouse_events import MouseEventType
from prompt_toolkit.widgets import Frame

class ScrollableBufferControl(BufferControl):
    def __init__(self, *args, **kwargs):
        self.layout = kwargs.pop("layout", None)
        super().__init__(*args, **kwargs)

    def mouse_handler(self, mouse_event):
        win = getattr(self.layout, "history_main", None)
        repl = self.layout.repl if self.layout else None

        if mouse_event.event_type == MouseEventType.SCROLL_UP:
            self.buffer.cursor_up(count=3)
            if win and win.render_info:
                win.vertical_scroll = max(0, win.vertical_scroll - 3)
            if repl:
                repl.request_invalidate()
            return None

        elif mouse_event.event_type == MouseEventType.SCROLL_DOWN:
            self.buffer.cursor_down(count=3)
            if win and win.render_info:
                line_count = win.render_info.ui_content.line_count
                window_height = win.render_info.window_height
                max_scroll = max(0, line_count - window_height)
                win.vertical_scroll = min(max_scroll, win.vertical_scroll + 3)
            elif win:
                win.vertical_scroll += 3
            if repl:
                repl.request_invalidate()
            return None

        return super().mouse_handler(mouse_event)

from .styles import SimpleAnsiLexer
from ..theme import THEME_ACCENTS, _current_theme_name

_LOGO_LINES = [
    "##        ######    ######   ##    ##",
    "##       ##    ##  ##    ##  ###  ###",
    "##       ##    ##  ##    ##  ## ## ##",
    "##       ##    ##  ##    ##  ##    ##",
    "########  ######    ######   ##    ##",
]


class LoomLayout:
    def __init__(self, repl):
        self.repl = repl
        self.session_input_window = None
        self.history_main = None

    def build_welcome_layout(self):
        """Centered logo + bordered input box (OpenCode-style home screen)."""
        logo_window = Window(
            content=FormattedTextControl(self._get_logo_formatted),
            height=5,
            align=WindowAlign.CENTER,
            style="class:welcome-logo",
        )

        welcome_input = Window(
            content=BufferControl(buffer=self.repl.input_buffer),
            height=1,
            wrap_lines=True,
        )

        model_line = Window(
            content=FormattedTextControl(self._get_welcome_model_line),
            height=1,
            style="class:welcome-model",
        )

        input_frame = Frame(
            body=HSplit([welcome_input, model_line]),
            style="class:welcome-input",
        )

        tip = Window(
            content=FormattedTextControl(self._get_welcome_tip),
            height=1,
            align=WindowAlign.CENTER,
            style="class:welcome-tip",
        )

        # Center block
        center = HSplit(
            [
                input_frame,
                Window(height=2),
                tip,
            ],
            width=Dimension(preferred=70, max=80),
        )

        return HSplit(
            [
                Window(height=Dimension(weight=2)),  # Top spacer (push down)
                logo_window,
                Window(height=2),
                VSplit(
                    [
                        Window(width=Dimension(weight=1)),  # Left spacer
                        center,
                        Window(width=Dimension(weight=1)),  # Right spacer
                    ]
                ),
                Window(height=Dimension(weight=3)),  # Bottom spacer
            ],
            style="class:history",
        )

    def build_session_layout(self):
        """Split-pane layout: history+input on the left, sidebar on the right."""
        # History area
        self.history_main = Window(
            content=ScrollableBufferControl(
                buffer=self.repl.history_buffer,
                lexer=SimpleAnsiLexer(
                    state_provider=lambda: getattr(self.repl, "is_modal_open", False)
                ),
                layout=self,
            ),
            wrap_lines=True,
            always_hide_cursor=True,
            style="class:history",
        )

        history_window = VSplit(
            [
                Window(width=2, style="class:history"),
                self.history_main,
                Window(width=2, style="class:history"),
            ]
        )

        # Session input area
        self.session_input_window = Window(
            content=BufferControl(buffer=self.repl.input_buffer),
            height=Dimension(min=1, max=3),
            wrap_lines=True,
        )

        input_model = Window(
            content=FormattedTextControl(self._get_input_model_line),
            height=1,
            style="class:input-box-model",
        )

        input_box_content = HSplit(
            [
                self.session_input_window,
                input_model,
            ],
            style="class:input-box",
        )

        input_box = VSplit(
            [
                Window(width=1, char=" ", style="class:input-box"),
                input_box_content,
                Window(width=1, char=" ", style="class:input-box"),
            ],
            style="class:input-box",
        )

        footer_left = Window(
            content=FormattedTextControl(self._get_session_footer_left),
            height=1,
            style="class:session-footer",
        )

        footer_right = Window(
            content=FormattedTextControl(self._get_session_footer_right),
            height=1,
            align=WindowAlign.RIGHT,
            style="class:session-footer",
        )

        input_area = HSplit(
            [
                Window(height=1, style="class:history"),
                VSplit(
                    [
                        Window(width=2, style="class:history"),
                        input_box,
                        Window(width=2, style="class:history"),
                    ]
                ),
                Window(height=1, style="class:history"),
                VSplit(
                    [
                        Window(width=2, style="class:history"),
                        footer_left,
                        footer_right,
                        Window(width=2, style="class:history"),
                    ]
                ),
            ]
        )

        left_pane = HSplit(
            [
                history_window,
                input_area,
            ]
        )

        return VSplit(
            [
                left_pane,
            ],
            style="class:history",
        )

    # ── Text Generators ───────────────────────────────────────────────────────

    def _get_logo_formatted(self):
        accent = THEME_ACCENTS.get(_current_theme_name, "#ffaf00")
        result = []
        for i, line in enumerate(_LOGO_LINES):
            if i > 0:
                result.append(("", "\n"))
            result.append((f"fg:{accent} bold", line))
        return result

    def _get_welcome_model_line(self):
        accent = THEME_ACCENTS.get(_current_theme_name, "#ffaf00")
        return [
            (f"fg:{accent} bold", "Build"),
            ("fg:#555566", " · "),
            ("bold #ffffff", f"{self.repl.ctx.config.model} "),
            ("fg:#555566", f"{self.repl.ctx.config.provider}"),
        ]

    def _get_welcome_tip(self):
        accent = THEME_ACCENTS.get(_current_theme_name, "#ffaf00")
        return [
            (f"fg:{accent}", "● "),
            (f"fg:{accent} bold", "Tip "),
            ("fg:#555566", "Type "),
            ("fg:#888899 bold", "/help"),
            ("fg:#555566", " to see all available commands"),
        ]



    def _get_input_model_line(self):
        accent = THEME_ACCENTS.get(_current_theme_name, "#ffaf00")
        return [
            (f"bg:#22222a fg:{accent} bold", "Build"),
            ("bg:#22222a fg:#555566", " · "),
            ("bg:#22222a #ffffff bold", f"{self.repl.ctx.config.model} "),
            ("bg:#22222a fg:#555566", f"{self.repl.ctx.config.provider}"),
        ]

    def _get_session_footer_left(self):
        ctx_usage = self.repl.ctx.state.get_context_usage(self.repl.ctx.config.model)
        res = [
            ("fg:#555577", f"{self.repl.ctx.state.tokens_used:,} ({ctx_usage:.0f}%)"),
            ("fg:#555566", " · "),
            ("fg:#555577", f"${self.repl.ctx.state.estimated_cost:.2f}"),
        ]
        if self.repl.is_working:
            duration = time.time() - self.repl.work_start_time
            from ..renderables import format_duration

            dur_str = format_duration(duration)
            res.extend(
                [
                    ("fg:#555566", " · "),
                    ("class:sidebar-header", f" Thinking {dur_str} "),
                ]
            )
        return res

    def _get_session_footer_right(self):
        accent = THEME_ACCENTS.get(_current_theme_name, "#ffaf00")
        cwd = os.path.basename(os.getcwd()) or "~"
        from ... import __version__

        base = []
        if getattr(self.repl, "toast_message", None):
            base.extend([("bg:#ff4444 fg:#ffffff bold", f" {self.repl.toast_message} "), ("fg:#555566", "   ")])

        base.extend([
            ("fg:#888899 bold", "esc "),
            ("fg:#555566", "interrupt   "),
            ("class:sidebar-value", f"/{cwd}  "),
            (f"fg:{accent}", "● "),
            (f"fg:{accent} bold", "Loom"),
            ("fg:#555566", f" {__version__}"),
        ])
        return base
