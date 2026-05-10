"""
Key bindings for the RouteCode REPL.

Extracted from app.py to reduce RouteCodeREPL to ~250 lines.
RouteCodeREPL inherits from KeyBindingsMixin.
"""

import asyncio
import time
from prompt_toolkit.keys import Keys
from prompt_toolkit.filters import has_focus
from ...utils.logger import get_logger

logger = get_logger(__name__)


class KeyBindingsMixin:
    """Mixin that provides _setup_key_bindings() for RouteCodeREPL."""

    def _setup_key_bindings(self):
        @self._kb.add("c-c")
        def _(event):
            if getattr(self, "is_working", False):
                if hasattr(self, "_current_agent_task") and self._current_agent_task:
                    self._current_agent_task.cancel()
                    self._rich_console.print(" [yellow]Agent aborted by user.[/yellow]")
                return

            now = time.time()
            if self._ctrl_c_press_time and (now - self._ctrl_c_press_time) < 3.0:
                event.app.exit()
            else:
                self._ctrl_c_press_time = now
                self.toast_message = "Press Ctrl+C again to exit"
                self.request_invalidate()

                def clear_toast():
                    if (
                        self.toast_message
                        and time.time() - self._ctrl_c_press_time >= 2.9
                    ):
                        self.toast_message = None
                        self.request_invalidate()

                if getattr(self, "ctx", None) and getattr(self.ctx, "loop", None):
                    self.ctx.loop.call_later(3.0, clear_toast)

        @self._kb.add("enter", filter=has_focus(self.input_buffer))
        def _(event):
            text = self.input_buffer.text.strip()
            self.input_buffer.reset()
            if text:
                if self._welcome_mode:
                    self._switch_to_session_mode()
                self._current_agent_task = asyncio.create_task(
                    self.handle_input(text)
                )

        @self._kb.add(Keys.ScrollUp)
        def _(event):
            self.history_buffer.cursor_up(count=3)
            win = self.layout_manager.history_main
            if win:
                win.vertical_scroll = max(0, win.vertical_scroll - 3)
            event.app.invalidate()

        @self._kb.add(Keys.ScrollDown)
        def _(event):
            self.history_buffer.cursor_down(count=3)
            win = self.layout_manager.history_main
            if win and win.render_info:
                max_scroll = max(
                    0,
                    win.render_info.ui_content.line_count
                    - win.render_info.window_height,
                )
                win.vertical_scroll = min(max_scroll, win.vertical_scroll + 3)
            elif win:
                win.vertical_scroll += 3
            event.app.invalidate()

        @self._kb.add(Keys.PageUp)
        def _(event):
            self.history_buffer.cursor_up(count=15)
            win = self.layout_manager.history_main
            if win:
                win.vertical_scroll = max(0, win.vertical_scroll - 15)
            event.app.invalidate()

        @self._kb.add(Keys.PageDown)
        def _(event):
            self.history_buffer.cursor_down(count=15)
            win = self.layout_manager.history_main
            if win and win.render_info:
                max_scroll = max(
                    0,
                    win.render_info.ui_content.line_count
                    - win.render_info.window_height,
                )
                win.vertical_scroll = min(max_scroll, win.vertical_scroll + 15)
            elif win:
                win.vertical_scroll += 15
            event.app.invalidate()
