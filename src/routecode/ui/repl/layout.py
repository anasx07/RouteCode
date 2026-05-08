import os
import time
from prompt_toolkit.layout import HSplit, VSplit, Window, WindowAlign, Dimension
from prompt_toolkit.layout.controls import BufferControl, FormattedTextControl
from prompt_toolkit.mouse_events import MouseEventType
from prompt_toolkit.widgets import Frame
from prompt_toolkit.filters import Condition
from .styles import SimpleAnsiLexer
from ..theme import THEME_ACCENTS, _current_theme_name
from ..renderables import get_logo, format_duration
from ... import __version__


class ModalAwareBufferControl(BufferControl):
    """BufferControl that blocks mouse interaction when a modal is open."""

    def __init__(self, *args, repl_ref=None, **kwargs):
        self._repl_ref = repl_ref
        super().__init__(*args, **kwargs)

    def mouse_handler(self, mouse_event):
        if self._repl_ref and getattr(self._repl_ref, "is_modal_open", False):
            return None
        return super().mouse_handler(mouse_event)


class ScrollableBufferControl(ModalAwareBufferControl):
    def __init__(self, *args, layout=None, **kwargs):
        self.layout = layout
        super().__init__(
            *args, repl_ref=getattr(layout, "repl", None) if layout else None, **kwargs
        )

    def mouse_handler(self, mouse_event):
        if super().mouse_handler(mouse_event) is None:
            return None

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


# ── Blocky pixel-art logo (OpenCode-style) ──────────────────────────────
# Single source of truth lives in renderables.get_logo()
_LOGO_ROUTE, _LOGO_CODE = get_logo()


class RouteCodeLayout:
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
        is_not_modal = Condition(lambda: not getattr(self.repl, "is_modal_open", False))

        self.history_main = Window(
            content=ScrollableBufferControl(
                buffer=self.repl.history_buffer,
                lexer=SimpleAnsiLexer(
                    state_provider=lambda: getattr(self.repl, "is_modal_open", False)
                ),
                layout=self,
                focusable=is_not_modal,
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
            content=ModalAwareBufferControl(
                buffer=self.repl.input_buffer,
                focusable=is_not_modal,
                repl_ref=self.repl,
            ),
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
        dim_color = "#888899"  # vibrant silver for "route"
        gap = "   "  # 3-space gap between words
        result = []
        for i in range(len(_LOGO_ROUTE)):
            if i > 0:
                result.append(("", "\n"))
            result.append((f"fg:{dim_color} bold", _LOGO_ROUTE[i]))
            result.append(("", gap))
            result.append((f"fg:{accent} bold", _LOGO_CODE[i]))

        # Add version below with a subtle offset for balance
        short_version = __version__.split("+")[0].split(".dev")[0]
        result.append(("", "\n\n"))
        result.append(("fg:#444455", f"v{short_version}"))
        return result

    def _get_welcome_model_line(self):
        accent = THEME_ACCENTS.get(_current_theme_name, "#ffaf00")
        return [
            (f"fg:{accent} bold", f"{self.repl.ctx.config.provider.title()}"),
            ("fg:#555566", " · "),
            ("bold #ffffff", f"{self.repl.ctx.config.model}"),
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
            (
                f"bg:#22222a fg:{accent} bold",
                f"{self.repl.ctx.config.provider.title()}",
            ),
            ("bg:#22222a fg:#555566", " · "),
            ("bg:#22222a #ffffff bold", f"{self.repl.ctx.config.model}"),
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
            # Industry Standard: Dynamic spinner frames for high-end feel
            frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]
            frame_idx = int(time.time() * 10) % len(frames)
            spinner = frames[frame_idx]

            dur_str = format_duration(duration)
            accent = THEME_ACCENTS.get(_current_theme_name, "#ffaf00")
            res.extend(
                [
                    ("fg:#555566", " · "),
                    (f"fg:{accent} bold", f"{spinner} "),
                    ("class:sidebar-header", f"Working {dur_str} "),
                ]
            )
        return res

    def _get_session_footer_right(self):
        accent = THEME_ACCENTS.get(_current_theme_name, "#ffaf00")
        cwd = os.path.basename(os.getcwd()) or "~"

        base = []
        if getattr(self.repl, "toast_message", None):
            base.extend(
                [
                    ("bg:#ff4444 fg:#ffffff bold", f" {self.repl.toast_message} "),
                    ("fg:#555566", "   "),
                ]
            )

        base.extend(
            [
                ("fg:#888899 bold", "esc "),
                ("fg:#555566", "interrupt   "),
                ("class:sidebar-value", f"/{cwd}  "),
                (f"fg:{accent}", "● "),
                (f"fg:{accent} bold", "RouteCode"),
                ("fg:#555566", f" v{__version__.split('+')[0].split('.dev')[0]}"),
            ]
        )
        return base
