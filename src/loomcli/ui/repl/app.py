import os
import shutil
import sys
import time
import asyncio
from io import StringIO
from prompt_toolkit.completion import WordCompleter
from prompt_toolkit.styles import DynamicStyle
from prompt_toolkit.output.color_depth import ColorDepth
from prompt_toolkit.cursor_shapes import CursorShape, SimpleCursorShapeConfig
from prompt_toolkit.data_structures import Size
from prompt_toolkit.application import Application
from prompt_toolkit.buffer import Buffer
from prompt_toolkit.layout import Layout, FloatContainer, Float
from prompt_toolkit.layout.containers import DynamicContainer
from prompt_toolkit.layout.menus import CompletionsMenu
from prompt_toolkit.key_binding import KeyBindings
from prompt_toolkit.keys import Keys
from prompt_toolkit.filters import has_focus

from ...core import SessionState, LoomContext, bus, PathGuard
from ...utils.logger import get_logger
from .. import console, get_tool_label
from ...commands import execute_command, get_command_metadata
from ...tools import registry, AuthorizationMiddleware
from ...config import config, CONFIG_DIR, compute_system_prompt
from ...domain.task_manager import task_manager
from ...core.orchestrator import AgentOrchestrator

from .styles import LoomVt100Output, build_repl_style
from .layout import LoomLayout
from .handlers import AppHooks

logger = get_logger(__name__)


class LoomREPL:
    def __init__(self):
        command_metadata = get_command_metadata()
        from ...domain.skills import discover_skills

        skill_commands = {}
        for skill_name in discover_skills():
            skill_commands[f"/{skill_name}"] = "Run user-defined skill"
        all_commands = {**command_metadata, **skill_commands}
        self.completer = WordCompleter(
            list(all_commands.keys()),
            meta_dict=all_commands,
            ignore_case=True,
            sentence=True,
        )

        self.history_buffer = Buffer(read_only=False)
        self.input_buffer = Buffer(
            multiline=False, completer=self.completer, complete_while_typing=True
        )

        # ── Flags ─────────────────────────────────────────────────────────────
        self._welcome_mode = True
        self.is_working = False
        self.work_start_time = 0

        # Setup redirection for rich console
        self._output_buffer = StringIO()
        self._rich_console = console  # The shared console

        # Disable litellm's verbose logging
        import litellm

        litellm.set_verbose = False
        litellm.suppress_debug_info = True

        import logging

        logging.getLogger("LiteLLM").setLevel(logging.ERROR)
        logging.getLogger("litellm").setLevel(logging.ERROR)

        # Force colors and reasonable width
        self._rich_console.force_terminal = True
        self._rich_console.color_system = "truecolor"
        try:
            self._rich_console.width = os.get_terminal_size().columns
        except Exception:
            self._rich_console.width = 120

        # Output interception
        self._original_print = self._rich_console.print
        self._rich_console.print = self._intercepted_print
        self.history_buffer.text = ""

        self.style = build_repl_style()
        self._set_terminal_background()

        from ...core.memory import MemoryManager

        self.memory = MemoryManager(CONFIG_DIR)
        self.state = SessionState()
        self.path_guard = PathGuard()
        self.ctx = LoomContext(
            state=self.state,
            config=config,
            console=self._rich_console,
            task_manager=task_manager,
            memory=self.memory,
            path_guard=self.path_guard,
        )
        self.auto_save_counter = 0
        self.logo_animation_count = 0
        self.orchestrator = AgentOrchestrator(self.ctx)
        from ...core.audit import audit_hook

        registry.add_post_hook(audit_hook)
        registry.add_middleware(AuthorizationMiddleware(confirm_callback=self._confirm_destructive))

        self._setup_event_handlers()
        self._kb = KeyBindings()
        self._setup_key_bindings()

        self.app = None
        self.layout_manager = LoomLayout(self)
        self.is_modal_open = False
        self._last_invalidate_time = 0.0
        self._invalidate_pending = False
        self.toast_message = None
        self._ctrl_c_press_time = 0.0

    def request_invalidate(self):
        if not self.app or not getattr(self, "ctx", None) or not self.ctx.loop:
            return

        def _do_invalidate():
            self._invalidate_pending = False
            self._last_invalidate_time = time.time()
            if self.app:
                self.app.invalidate()

        def _schedule():
            now = time.time()
            if now - self._last_invalidate_time > 0.02:
                self._last_invalidate_time = now
                if self.app:
                    self.app.invalidate()
            else:
                if not self._invalidate_pending:
                    self._invalidate_pending = True
                    self.ctx.loop.call_later(0.02, _do_invalidate)

        try:
            current_loop = asyncio.get_running_loop()
            if current_loop is self.ctx.loop:
                _schedule()
            else:
                self.ctx.loop.call_soon_threadsafe(_schedule)
        except RuntimeError:
            # We are not in an async context, safely delegate to the main loop
            self.ctx.loop.call_soon_threadsafe(_schedule)

    def _is_scrolled_to_bottom(self):
        return self.history_buffer.cursor_position >= len(self.history_buffer.text)

    def update_style(self):
        from . import styles
        self.style = styles.build_repl_style(is_dimmed=self.is_modal_open)
        self.request_invalidate()

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
                    if self.toast_message and time.time() - self._ctrl_c_press_time >= 2.9:
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
                self._current_agent_task = asyncio.create_task(self.handle_input(text))

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
                max_scroll = max(0, win.render_info.ui_content.line_count - win.render_info.window_height)
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
                max_scroll = max(0, win.render_info.ui_content.line_count - win.render_info.window_height)
                win.vertical_scroll = min(max_scroll, win.vertical_scroll + 15)
            elif win:
                win.vertical_scroll += 15
            event.app.invalidate()

    def _intercepted_print(self, *args, **kwargs):
        with self._rich_console.capture() as capture:
            self._original_print(*args, **kwargs)
        captured = capture.get()
        was_at_bottom = self._is_scrolled_to_bottom()
        old_cursor = self.history_buffer.cursor_position
        self.history_buffer.cursor_position = len(self.history_buffer.text)
        self.history_buffer.insert_text(captured)
        if not was_at_bottom:
            self.history_buffer.cursor_position = old_cursor
        self.request_invalidate()

    def _setup_event_handlers(self):
        from ...utils.notify import notify_task_complete

        bus.on(
            "task.completed",
            lambda task_id, description, **kwargs: notify_task_complete(
                task_id, description
            ),
        )

        async def _on_turn_complete(count, **kwargs):
            if count > 0 and count % 5 == 0:
                from ...commands import handle_save

                await handle_save(["auto"], self.ctx)

        bus.on("session.turn_complete", _on_turn_complete)
        bus.on("session.reset", self._on_session_reset)
        bus.on("ui.theme_changed", lambda **kwargs: self._on_theme_changed())

    async def _periodic_refresh_loop(self):
        """Background task to keep the UI alive and spinning while working."""
        while True:
            if getattr(self, "is_working", False) or getattr(self, "toast_message", None):
                self.request_invalidate()
            await asyncio.sleep(0.1)

    def _on_session_reset(self, **kwargs):
        self.history_buffer.text = ""
        self._welcome_mode = True
        self.request_invalidate()

    def _on_theme_changed(self):
        self._set_terminal_background()
        self.style = build_repl_style()
        self.request_invalidate()

    def _set_terminal_background(self):
        self.style = build_repl_style()

    def _on_resize(self):
        try:
            self._rich_console.width = os.get_terminal_size().columns
        except Exception:
            pass

    async def run(self):
        self.ctx.loop = asyncio.get_running_loop()

        # Pre-build both layouts
        self._welcome_container = self.layout_manager.build_welcome_layout()
        self._session_container = self.layout_manager.build_session_layout()

        root_container = FloatContainer(
            content=DynamicContainer(self._get_active_layout),
            floats=[
                Float(
                    xcursor=True, ycursor=True, content=CompletionsMenu(max_height=16)
                )
            ],
        )

        self.app = Application(
            layout=Layout(root_container, focused_element=self.input_buffer),
            key_bindings=self._kb,
            style=DynamicStyle(lambda: self.style),
            mouse_support=True,
            full_screen=True,
            cursor=SimpleCursorShapeConfig(CursorShape.BLOCK),
            color_depth=ColorDepth.TRUE_COLOR,
            output=LoomVt100Output(
                sys.stdout,
                lambda: Size(
                    rows=shutil.get_terminal_size().lines,
                    columns=shutil.get_terminal_size().columns,
                ),
                state_provider=lambda: getattr(self, 'is_modal_open', False)
            ),
        )
        
        self.app.loom_repl = self

        self.app.loom_repl = self
        
        # Start periodic refresh loop for animations
        asyncio.create_task(self._periodic_refresh_loop())

        await self.app.run_async()

    def _get_active_layout(self):
        if self._welcome_mode:
            return self._welcome_container
        else:
            return self._session_container

    def _switch_to_session_mode(self):
        self._welcome_mode = False
        if self.app:
            if self.layout_manager.session_input_window:
                self.app.layout.focus(self.layout_manager.session_input_window)
            self.request_invalidate()

    async def handle_input(self, text):
        self.history_buffer.cursor_position = len(self.history_buffer.text)
        if text.startswith("/"):
            if await execute_command(text, self.ctx):
                pass
            else:
                self._rich_console.print(f" [error]✘[/error] Unknown command: {text}")
        else:
            await self.process_agent_request(text)

    async def process_agent_request(self, user_input: str):
        if not self.orchestrator.provider:
            self.orchestrator.refresh_provider()

        await compute_system_prompt(self.ctx)
        self.ctx.state.session_messages.append({"role": "user", "content": user_input})
        from ..renderables import print_user_message
        print_user_message(user_input, target_console=self._rich_console)

        hooks = AppHooks(self)
        hooks.start_time = time.time()
        hooks._add_thinking()

        async def tool_executor(name, args):
            return await self.call_tool(name, args)

        self.is_working = True
        self.work_start_time = time.time()
        self.request_invalidate()

        try:
            await self.orchestrator.run(
                self.ctx.state.session_messages,
                hooks=hooks,
                tool_executor=tool_executor,
            )
        except Exception as e:
            self._rich_console.print(f" [error]✘[/error] Error: {e}")
        finally:
            self.is_working = False
            self.request_invalidate()

    async def call_tool(self, name: str, arguments: dict):
        return await registry.execute_tool(
            name, arguments, ctx=self.ctx, provider=self.orchestrator.provider
        )

    async def _confirm_destructive(self, tool, args: dict) -> bool:
        from .. import print_diff, LoomDialog
        import difflib

        label = get_tool_label(tool.name, args)
        if tool.name == "file_edit":
            old_str = args.get("old_string", "")
            new_str = args.get("new_string", "")
            if old_str and new_str:
                old_lines = old_str.splitlines(keepends=True)
                new_lines = new_str.splitlines(keepends=True)
                diff_lines = list(
                    difflib.unified_diff(
                        old_lines, new_lines, fromfile="original", tofile="proposed"
                    )
                )
                print_diff("".join(diff_lines))

        buttons = [
            ("Allow this time", "allow"),
            ("Allow this session", "session_allow"),
            ("Always Allow", "always_allow"),
            ("Deny", "deny"),
        ]
        
        # Security: Remove "Always Allow" for extremely high-risk tools like bash
        if tool.name == "bash":
            buttons = [b for b in buttons if b[1] != "always_allow"]

        dialog = LoomDialog(
            title="Security Confirmation",
            text=f"The agent wants to run a destructive tool: [bold yellow]{label}[/bold yellow].\nAllow this operation?",
            buttons=buttons,
        )
        result = await dialog.run_async()
        if result == "allow":
            return True
        if result == "session_allow":
            allowlist = self.ctx.state.session_allowlist
            pattern = f"{tool.name}(*)"
            if pattern not in allowlist:
                allowlist.append(pattern)
            return True
        if result == "always_allow":
            allowlist = self.ctx.config.allowlist or []
            pattern = f"{tool.name}(*)"
            if pattern not in allowlist:
                allowlist.append(pattern)
                self.ctx.config.allowlist = allowlist
                await self.ctx.config.save_async()
            return True
        return False
