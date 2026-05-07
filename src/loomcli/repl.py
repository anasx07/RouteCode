import os
import getpass
import json
import time
import asyncio
import concurrent.futures
from prompt_toolkit import PromptSession
from prompt_toolkit.history import FileHistory
from prompt_toolkit.completion import WordCompleter
from prompt_toolkit.lexers import Lexer
from prompt_toolkit.styles import Style, DynamicStyle
from prompt_toolkit.formatted_text import to_formatted_text, ANSI
from prompt_toolkit.mouse_events import MouseEvent, MouseEventType
from prompt_toolkit.output.color_depth import ColorDepth
from rich.markdown import Markdown
from rich.live import Live

from .core import get_logger
logger = get_logger(__name__)

from . import ui as _ui
from .ui import (
    console, print_error, print_welcome_screen, 
    print_thought_elapsed, print_status_line,
    print_tool_call, print_tool_result, print_session_stats,
    get_thinking_indicator, LoadingRenderable, get_tool_label,
    LoomFace, LOOM_FACES, print_step, get_theme_bg
)
from .commands import execute_command, get_command_metadata
from .tools import registry
from .tools.base import BaseTool
from .config import config, CONFIG_DIR
from .core import SessionState, count_tokens
from .system_prompt import compute_system_prompt
from .core import classify_exception
from .agents.registry import PROVIDER_MAP
from .utils import parse_hex_color, strip_thought
from .core import LoomContext
from .task_manager import task_manager
from .core import bus
from .orchestrator import AgentOrchestrator, OrchestratorHooks

from prompt_toolkit.output.vt100 import Vt100_Output

class LoomVt100Output(Vt100_Output):
    """
    Custom VT100 output that ensures the theme background color is preserved
    during screen clearing and attribute resets.
    """
    def erase_end_of_line(self):
        bg = get_theme_bg()
        try:
            r, g, b = parse_hex_color(bg)
            self.write_raw(f"\033[48;2;{r};{g};{b}m")
        except Exception:
            pass
        super().erase_end_of_line()

    def reset_attributes(self):
        super().reset_attributes()
        bg = get_theme_bg()
        try:
            r, g, b = parse_hex_color(bg)
            self.write_raw(f"\033[48;2;{r};{g};{b}m")
        except Exception:
            pass





from io import StringIO
from prompt_toolkit.application import Application
from prompt_toolkit.buffer import Buffer
from prompt_toolkit.layout import Layout, HSplit, VSplit, Window, BufferControl, FloatContainer, Float
from prompt_toolkit.layout.controls import BufferControl, FormattedTextControl
from prompt_toolkit.layout.menus import CompletionsMenu
from prompt_toolkit.lexers import Lexer
from prompt_toolkit.layout.dimension import Dimension
from prompt_toolkit.layout.processors import PasswordProcessor
from prompt_toolkit.widgets import Frame, TextArea, SearchToolbar
from prompt_toolkit.key_binding import KeyBindings
from prompt_toolkit.filters import has_focus, is_searching

class SimpleAnsiLexer(Lexer):
    def lex_document(self, document):
        def get_line(i):
            return ANSI(document.lines[i]).__pt_formatted_text__()
        return get_line

class LoomREPL:
    def __init__(self):
        command_metadata = get_command_metadata()
        from .skills import discover_skills
        skill_commands = {}
        for skill_name in discover_skills():
            skill_commands[f"/{skill_name}"] = "Run user-defined skill"
        all_commands = {**command_metadata, **skill_commands}
        self.completer = WordCompleter(
            list(all_commands.keys()),
            meta_dict=all_commands,
            ignore_case=True,
            sentence=True
        )

        self.history_buffer = Buffer(read_only=False)
        self.input_buffer = Buffer(
            multiline=False, 
            completer=self.completer,
            complete_while_typing=True
        )
        
        # Setup redirection for rich console
        self._output_buffer = StringIO()
        self._rich_console = console # The shared console
        # Force colors and reasonable width for the intercepted console
        self._rich_console.force_terminal = True
        self._rich_console.color_system = "truecolor"
        try:
            self._rich_console.width = os.get_terminal_size().columns
        except:
            self._rich_console.width = 120
            
        # We'll use a hook to capture output
        self._original_print = self._rich_console.print
        self._rich_console.print = self._intercepted_print
        self.history_buffer.text = "" # Clear any initial junk

        self.style = Style.from_dict({
            "":                "bg:#1a1a2e #ffffff",
            "history":         "bg:#1a1a2e",
            "input-area":      "bg:#1a1a2e",
            "status-bar":      "bg:#161625 #aaaaaa",
            "status-bar.workspace": "fg:#ffffff bold",
            "status-bar.model": "fg:#ffaf00",
            "status-bar.metrics": "fg:#666666",
            "prompt":          "fg:#ffaf00 bold",
            "divider":         "fg:#2a2a40",
        })
        self._set_terminal_background()

        from .memory import MemoryManager
        self.memory = MemoryManager(CONFIG_DIR)
        self.state = SessionState()
        from .core import PathGuard
        self.path_guard = PathGuard()
        self.ctx = LoomContext(
            state=self.state,
            config=config,
            console=self._rich_console,
            task_manager=task_manager,
            memory=self.memory,
            path_guard=self.path_guard
        )
        self.auto_save_counter = 0
        self.logo_animation_count = 0
        self.orchestrator = AgentOrchestrator(self.ctx)
        from .core.audit import audit_hook
        registry.add_post_hook(audit_hook)
        self._setup_event_handlers()

        self._kb = KeyBindings()
        @self._kb.add("c-c")
        def _(event):
            event.app.exit()

        @self._kb.add("enter", filter=has_focus(self.input_buffer))
        def _(event):
            text = self.input_buffer.text.strip()
            self.input_buffer.reset()
            if text:
                asyncio.create_task(self.handle_input(text))

        self.app = None

    def _intercepted_print(self, *args, **kwargs):
        # Capture the output to string
        with console.capture() as capture:
            self._original_print(*args, **kwargs)
        captured = capture.get()
        # Append to history buffer
        self.history_buffer.insert_text(captured)
        # Move cursor to end to trigger auto-scroll
        self.history_buffer.cursor_position = len(self.history_buffer.text)
        if self.app:
            self.app.invalidate()

    def _setup_event_handlers(self):
        from .notify import notify_task_complete
        bus.on("task.completed", lambda task_id, description, **kwargs: notify_task_complete(task_id, description))
        
        async def _on_turn_complete(count, **kwargs):
            if count > 0 and count % 5 == 0:
                from .commands import handle_save
                await handle_save(["auto"], self.ctx)
        bus.on("session.turn_complete", _on_turn_complete)
        bus.on("ui.theme_changed", lambda **kwargs: self._set_terminal_background())

    def _get_status_bar_text(self):
        self._on_resize()
        # workspace (/directory)  /model gemini-3.1-flash-preview  quota % used  context 6% used  memory 385.3 MB
        try:
            import psutil
            process = psutil.Process(os.getpid())
            mem_info = process.memory_info().rss / (1024 * 1024)
        except ImportError:
            mem_info = None
        
        ctx_usage = self.ctx.state.get_context_usage(self.ctx.config.model)
        
        parts = [
            ("class:status-bar.workspace", f"  {os.path.basename(os.getcwd())}  "),
            ("class:status-bar.model", f" {self.ctx.config.model}  "),
            ("class:status-bar.metrics", f"context {ctx_usage:.0f}%  "),
        ]
        if mem_info is not None:
            parts.append(("class:status-bar.metrics", f"mem {mem_info:.1f} MB  "))
            
        return parts

    def _set_terminal_background(self):
        bg = get_theme_bg()
        # Ensure we keep our navy bg if it's the default dark theme, or use theme bg
        self.style = Style.from_dict({
            "":                f"bg:{bg} #ffffff",
            "history":         f"bg:{bg}",
            "input-area":      f"bg:{bg}",
            "status-bar":      "bg:#161625 #aaaaaa",
            "status-bar.workspace": "fg:#ffffff bold",
            "status-bar.model": "fg:#ffaf00",
            "status-bar.metrics": "fg:#666666",
            "prompt":          "fg:#ffaf00 bold",
            "divider":         "fg:#2a2a40",
            "accent":          "#ffaf00",
        })

    def _on_resize(self):
        try:
            self._rich_console.width = os.get_terminal_size().columns
        except:
            pass

    async def run(self):
        import asyncio
        self.ctx.loop = asyncio.get_running_loop()
        
        # Build Layout
        # History window with padding
        history_main = Window(
            content=BufferControl(buffer=self.history_buffer, lexer=SimpleAnsiLexer()),
            wrap_lines=True,
            always_hide_cursor=True,
            style="class:history"
        )
        
        history_window = VSplit([
            Window(width=4, style="class:history"), # Left Padding
            history_main,
            Window(width=4, style="class:history"), # Right Padding
        ])
        
        input_window = Window(
            content=BufferControl(buffer=self.input_buffer),
            height=Dimension(min=1, max=3),
            wrap_lines=True
        )
        
        status_bar = Window(
            content=FormattedTextControl(self._get_status_bar_text),
            height=1,
            style="class:status-bar"
        )
        
        body = HSplit([
            history_window,
            Window(height=1, char="\u2500", style="class:divider"), # Divider
            VSplit([
                Window(width=6, content=FormattedTextControl([("class:prompt", "    > ")])),
                input_window,
            ], height=Dimension(min=1, max=3), style="class:input-area"),
            status_bar
        ], style="class:history")

        root_container = FloatContainer(
            content=body,
            floats=[
                Float(
                    xcursor=True,
                    ycursor=True,
                    content=CompletionsMenu(max_height=16)
                )
            ]
        )
        
        self.app = Application(
            layout=Layout(root_container, focused_element=input_window),
            key_bindings=self._kb,
            style=DynamicStyle(lambda: self.style),
            mouse_support=True,
            full_screen=True,
            color_depth=ColorDepth.TRUE_COLOR
        )
        
        # Initial output
        from .ui import print_welcome_screen
        user_name = getpass.getuser()
        print_welcome_screen(user_name, self.ctx.config.model, self.ctx.config.provider)
        
        await self.app.run_async()

    async def handle_input(self, text):
        if text.startswith("/"):
            if await execute_command(text, self.ctx):
                # Provider refresh etc.
                pass
            else:
                self._rich_console.print(f" [error]✘[/error] Unknown command: {text}")
        else:
            await self.process_agent_request(text)

    async def process_agent_request(self, user_input: str):
        # [Existing logic from process_agent_request remains mostly the same, 
        # but outputs to self._rich_console which is intercepted]
        
        if not self.orchestrator.provider:
            self.orchestrator.refresh_provider()

        from .system_prompt import compute_system_prompt
        system_content = await compute_system_prompt(self.ctx)
        
        messages = self.ctx.state.session_messages.to_list()
        if messages and messages[0].get("role") == "system":
            messages[0]["content"] = system_content
        else:
            messages.insert(0, {"role": "system", "content": system_content})
        
        self.ctx.state.session_messages.set_messages(messages)
        self.ctx.state.session_messages.append({"role": "user", "content": user_input})
        
        class AppHooks(OrchestratorHooks):
            def __init__(self, repl):
                self.repl = repl
                self.start_time = None
                self.full_response = ""

            async def on_chunk(self, chunk):
                if chunk["type"] == "text":
                    content = chunk["content"]
                    self.full_response += content
                    # Simple streaming: just print the chunk
                    # (Note: this might show <thought> tags if they are in the stream)
                    self.repl._rich_console.print(content, end="")

            async def on_turn_complete(self, full_response, tool_calls):
                elapsed = time.time() - self.start_time
                # We already streamed most of it, but rich.print(Markdown) 
                # looks better for the final result. 
                # However, to avoid duplication, we might just print a newline 
                # or the session stats.
                self.repl._rich_console.print("\n")
                
                print_status_line(self.repl.ctx.config.model, elapsed)
                print_session_stats(self.repl.ctx.state)

            async def on_tool_call(self, name, args):
                print_tool_call(name, args)

            async def on_tool_result(self, name, result, elapsed):
                print_tool_result(result, elapsed, name)
                if name == "file_edit" and result.get("success") and result.get("diff"):
                    print_diff(result["diff"])

        hooks = AppHooks(self)
        hooks.start_time = time.time()
        
        async def tool_executor(name, args):
            result = await self.call_tool(name, args)
            return result

        try:
            await self.orchestrator.run(self.ctx.state.session_messages, hooks=hooks, tool_executor=tool_executor)
        except Exception as e:
            self._rich_console.print(f" [error]✘[/error] Error: {e}")

    def _on_context_warning(self, usage: float, type: str, **kwargs):
        if type == "full":
            self._rich_console.print(f" [warning]⚠[/warning] [bold red]Context critical: {usage:.1f}%![/bold red]")
        else:
            self._rich_console.print(f" [warning]⚠[/warning] [dim]Context usage: {usage:.1f}%[/dim]")

    def _logoTooltip(self):
        self.logo_animation_count += 1

    def _get_git_context(self) -> str:
        from .git import get_git_context
        return get_git_context()

    async def _confirm_destructive(self, tool: BaseTool, args: dict) -> bool:
        if not tool.isDestructive:
            return True
        if self._check_permission_allow(tool, args):
            return True
        if self._check_permission_deny(tool, args):
            self._rich_console.print(f" [error]✘[/error] [dim]Blocked by permission rules: {tool.name}[/dim]")
            return False

        from .ui import get_tool_label, print_diff, LoomDialog
        import difflib
        label = get_tool_label(tool.name, args)
        if tool.name == "file_edit":
            old_str = args.get("old_string", "")
            new_str = args.get("new_string", "")
            if old_str and new_str:
                old_lines = old_str.splitlines(keepends=True)
                new_lines = new_str.splitlines(keepends=True)
                diff_lines = list(difflib.unified_diff(old_lines, new_lines, fromfile="original", tofile="proposed"))
                print_diff("".join(diff_lines))

        dialog = LoomDialog(
            title="Destructive Tool",
            text=f"Allow {label}?",
            buttons=[("Allow", "allow"), ("Deny", "deny"), ("Always Allow", "always_allow")]
        )
        result = await dialog.run_async()
        if result == "allow":
            return True
        if result == "always_allow":
            allowlist = self.ctx.config.allowlist or []
            pattern = f"{tool.name}(*)"
            if pattern not in allowlist:
                allowlist.append(pattern)
                self.ctx.config.allowlist = allowlist
                await self.ctx.config.save_async()
                self._rich_console.print(f" [success]✔[/success] [dim]Added {pattern} to allowlist.[/dim]")
            return True
        return False

    def _check_permission_allow(self, tool: BaseTool, args: dict) -> bool:
        from fnmatch import fnmatch
        allowlist = self.ctx.config.allowlist or []
        pattern = f"{tool.name}(*)"
        for rule in allowlist:
            if fnmatch(pattern, rule):
                return True
        return False

    def _check_permission_deny(self, tool: BaseTool, args: dict) -> bool:
        from fnmatch import fnmatch
        denylist = self.ctx.config.denylist or []
        pattern = f"{tool.name}(*)"
        for rule in denylist:
            if fnmatch(pattern, rule):
                return True
        return False

    async def call_tool(self, name: str, arguments: dict):
        import asyncio
        tool = registry.get_tool(name)
        if not tool:
            return {"error": f"Tool not found: {name}"}
        registry.run_pre_hooks(name, arguments)
        if tool.isDestructive and not await self._confirm_destructive(tool, arguments):
            registry.run_post_hooks(name, {"error": "Permission denied"})
            return {"error": "Permission denied by user"}
        try:
            result = await asyncio.to_thread(tool.execute, **arguments, ctx=self.ctx, provider=self.orchestrator.provider)
            registry.run_post_hooks(name, result)
            return result
        except Exception as e:
            result = {"error": str(e)}
            registry.run_post_hooks(name, result)
            return result
