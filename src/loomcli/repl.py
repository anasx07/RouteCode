import os
import getpass
import json
import time
import concurrent.futures
from prompt_toolkit import PromptSession
from prompt_toolkit.history import FileHistory
from prompt_toolkit.completion import WordCompleter
from prompt_toolkit.styles import Style, DynamicStyle
from prompt_toolkit.formatted_text import to_formatted_text
from prompt_toolkit.mouse_events import MouseEvent, MouseEventType
from prompt_toolkit.output.color_depth import ColorDepth
from rich.markdown import Markdown
from rich.live import Live

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
from .state import SessionState, count_tokens
from .system_prompt import compute_system_prompt
from .errors import classify_exception
from .agents.registry import PROVIDER_MAP
from .context import LoomContext
from .task_manager import task_manager
from .events import bus
from .orchestrator import AgentOrchestrator, OrchestratorHooks





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
        
        self.style = Style.from_dict({"": "bg:#1a1a2e"})  # placeholder, overwritten below
        self._set_terminal_background()
        
        # Force VT100 output so ANSI bg colors work (Win32Output ignores them)
        import sys as _sys
        try:
            from prompt_toolkit.output.vt100 import Vt100_Output
            from prompt_toolkit.output.defaults import create_output
            _win32_output = create_output()
            pt_output = Vt100_Output(_sys.stdout, lambda: _win32_output.get_size())
            
            # Monkey-patch erase_end_of_line to FORCE the background color to paint the rest of the line
            original_erase = pt_output.erase_end_of_line
            def _erase_with_bg():
                bg = get_theme_bg()
                try:
                    r, g, b = int(bg[1:3], 16), int(bg[3:5], 16), int(bg[5:7], 16)
                    pt_output.write_raw(f"\033[48;2;{r};{g};{b}m")
                except:
                    pass
                original_erase()
            pt_output.erase_end_of_line = _erase_with_bg
            
            # Also patch reset_attributes to prevent terminal default bg from leaking in
            original_reset = pt_output.reset_attributes
            def _reset_with_bg():
                original_reset()
                bg = get_theme_bg()
                try:
                    r, g, b = int(bg[1:3], 16), int(bg[3:5], 16), int(bg[5:7], 16)
                    pt_output.write_raw(f"\033[48;2;{r};{g};{b}m")
                except:
                    pass
            pt_output.reset_attributes = _reset_with_bg
        except Exception:
            pt_output = None
        
        history_file = CONFIG_DIR / "history"
        session_kwargs = dict(
            history=FileHistory(str(history_file)),
            completer=self.completer,
            complete_while_typing=True,
            bottom_toolbar=self._get_bottom_toolbar,
            style=DynamicStyle(lambda: self.style),
            mouse_support=True,
            color_depth=ColorDepth.TRUE_COLOR,
        )
        if pt_output:
import os
import getpass
import json
import time
import concurrent.futures
from prompt_toolkit import PromptSession
from prompt_toolkit.history import FileHistory
from prompt_toolkit.completion import WordCompleter
from prompt_toolkit.styles import Style, DynamicStyle
from prompt_toolkit.formatted_text import to_formatted_text
from prompt_toolkit.mouse_events import MouseEvent, MouseEventType
from prompt_toolkit.output.color_depth import ColorDepth
from rich.markdown import Markdown
from rich.live import Live

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
from .state import SessionState, count_tokens
from .system_prompt import compute_system_prompt
from .errors import classify_exception
from .agents.registry import PROVIDER_MAP
from .context import LoomContext
from .task_manager import task_manager
from .events import bus
from .orchestrator import AgentOrchestrator, OrchestratorHooks





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
        
        self.style = Style.from_dict({"": "bg:#1a1a2e"})  # placeholder, overwritten below
        self._set_terminal_background()
        
        # Force VT100 output so ANSI bg colors work (Win32Output ignores them)
        import sys as _sys
        try:
            from prompt_toolkit.output.vt100 import Vt100_Output
            from prompt_toolkit.output.defaults import create_output
            _win32_output = create_output()
            pt_output = Vt100_Output(_sys.stdout, lambda: _win32_output.get_size())
            
            # Monkey-patch erase_end_of_line to FORCE the background color to paint the rest of the line
            original_erase = pt_output.erase_end_of_line
            def _erase_with_bg():
                bg = get_theme_bg()
                try:
                    r, g, b = int(bg[1:3], 16), int(bg[3:5], 16), int(bg[5:7], 16)
                    pt_output.write_raw(f"\033[48;2;{r};{g};{b}m")
                except:
                    pass
                original_erase()
            pt_output.erase_end_of_line = _erase_with_bg
            
            # Also patch reset_attributes to prevent terminal default bg from leaking in
            original_reset = pt_output.reset_attributes
            def _reset_with_bg():
                original_reset()
                bg = get_theme_bg()
                try:
                    r, g, b = int(bg[1:3], 16), int(bg[3:5], 16), int(bg[5:7], 16)
                    pt_output.write_raw(f"\033[48;2;{r};{g};{b}m")
                except:
                    pass
            pt_output.reset_attributes = _reset_with_bg
        except Exception:
            pt_output = None
        
        history_file = CONFIG_DIR / "history"
        session_kwargs = dict(
            history=FileHistory(str(history_file)),
            completer=self.completer,
            complete_while_typing=True,
            bottom_toolbar=self._get_bottom_toolbar,
            style=DynamicStyle(lambda: self.style),
            mouse_support=True,
            color_depth=ColorDepth.TRUE_COLOR,
        )
        if pt_output:
            session_kwargs["output"] = pt_output
        self.session = PromptSession(**session_kwargs)
        self.provider = None
        self.messages = []
        from .memory import MemoryManager
        self.memory = MemoryManager(CONFIG_DIR)
        self.state = SessionState()
        self.ctx = LoomContext(
            state=self.state,
            config=config,
            console=console,
            task_manager=task_manager,
            memory=self.memory
        )
        self.auto_save_counter = 0
        self.logo_animation_count = 0
        self.orchestrator = AgentOrchestrator(self.ctx)
        self._setup_event_handlers()

    def _setup_event_handlers(self):
        """Register listeners for system events."""
        from .notify import notify_task_complete
        bus.on("task.completed", lambda task_id, description, **kwargs: notify_task_complete(task_id, description))
        
        async def _on_turn_complete(count, **kwargs):
            if count > 0 and count % 5 == 0:
                from .commands import handle_save
                await handle_save(["auto"], self.ctx)
        bus.on("session.turn_complete", _on_turn_complete)

        bus.on("ui.theme_changed", lambda **kwargs: self._set_terminal_background())

    def _logoTooltip(self):
        self.logo_animation_count += 1

    def _get_bottom_toolbar(self):
        from .task_manager import task_manager
        tasks = task_manager.list()
        running = [t for t in tasks if t["status"] == "running"]
        task_info = ""
        if running:
            count = len(running)
            desc = running[0]["description"][:30]
            task_info = f" | [accent]\u23f3[/accent] {count} task(s): {desc} " if count == 1 else f" | [accent]\u23f3[/accent] {count} tasks running "

        face = LOOM_FACES[self.logo_animation_count % len(LOOM_FACES)].strip()

        def _click_logo(me):
            if me.event_type == MouseEventType.MOUSE_DOWN:
                self.logo_animation_count += 2
                return None

        ft = to_formatted_text([
            ("class:toolbar", f" Build \u00b7 {self.ctx.config.model} "),
            ("class:toolbar", f" | {os.getcwd()} "),
            ("class:toolbar", task_info),
            ("class:accent", " LOOM ", _click_logo),
            ("class:toolbar", f" {face} "),
            ("class:toolbar", " | tab agents \u00b7 ctrl+p commands "),
        ])
        return ft

    def _initialize_provider(self):
        api_key = config.get_api_key()
        if not api_key:
            from .ui import LoomDialog
            dialog = LoomDialog(
                title="API Key Required",
                text=f"Please enter your API key for [accent]{config.provider}[/accent]:",
                dialog_type="input",
                password=True
            )
            # Use run() which handles the loop internally for sync context
            result = dialog.run()
            if result:
                api_key = result.strip()
                self.ctx.config.set_api_key(self.ctx.config.provider, api_key)
            else:
                return False
        
        provider_class = PROVIDER_MAP.get(self.ctx.config.provider)
        if provider_class:
            self.provider = provider_class(api_key)
            return True
        return False

    def _set_terminal_background(self):
        """Update prompt_toolkit style to match the active theme background."""
        bg = get_theme_bg()
        try:
            r = int(bg[1:3], 16)
            g = int(bg[3:5], 16)
            b = int(bg[5:7], 16)
            tr = min(r + 12, 255)
            tg = min(g + 12, 255)
            tb = min(b + 12, 255)
            toolbar_bg = f"#{tr:02x}{tg:02x}{tb:02x}"
        except (ValueError, IndexError):
            toolbar_bg = bg
        self.style = Style.from_dict({
            "":                      f"bg:{bg}",
            "prompt":                f"bg:{bg} #ffffff",
            "bottom-toolbar":        f"bg:{toolbar_bg} #aaaaaa",
            "bottom-toolbar.text":   f"bg:{toolbar_bg} #aaaaaa",
            "toolbar":               f"#aaaaaa bg:{toolbar_bg}",
            "accent":                "#ffaf00",
            "logo_clickable":        "bg:#ffaf00 #000000",
            "completion-menu":       f"bg:{toolbar_bg} #ffffff",
            "completion-menu.completion.current": "bg:#ffaf00 #000000",
            "scrollbar.background":  f"bg:{toolbar_bg}",
            "scrollbar.button":      f"bg:{bg}",
        })

    async def run(self):
        from .ui import apply_theme
        apply_theme(self.ctx.config.theme)
        self._set_terminal_background()
        self.state.start_time = time.time()
        self.state.session_messages = []
        import sys
        from .ui import get_theme_bg
        bg = get_theme_bg()
        r, g, b = int(bg[1:3], 16), int(bg[3:5], 16), int(bg[5:7], 16)
        sys.stdout.write(f"\033[48;2;{r};{g};{b}m\033[2J\033[H")
        sys.stdout.flush()
        
        print_welcome_screen(getpass.getuser(), self.ctx.config.model, self.ctx.config.provider)
        
        # Asynchronously load memory
        await self.memory._load_async()
        
        if not self._initialize_provider():
            print_error(f"No API key found for provider '{self.ctx.config.provider}'. Use [command]/config[/command] to set it.")

        system_content = compute_system_prompt(self.ctx)
        self.messages = [{"role": "system", "content": system_content}]

        last_size = None
        while True:
            try:
                try:
                    current_size = self.session.output.get_size()
                    if current_size != last_size:
                        from .ui import get_theme_bg, _set_terminal_bg
                        bg = get_theme_bg()
                        _set_terminal_bg(bg)
                        last_size = current_size
                except Exception:
                    pass

                def pre_run():
                    import sys
                    from .ui import get_theme_bg
                    bg = get_theme_bg()
                    try:
                        r, g, b = int(bg[1:3], 16), int(bg[3:5], 16), int(bg[5:7], 16)
                        sys.stdout.write(f"\r\033[48;2;{r};{g};{b}m\033[K")
                        sys.stdout.flush()
                    except Exception:
                        pass

                user_input = await self.session.prompt_async(
                    [("class:accent", "┃ "), ("class:prompt", "> ")],
                    pre_run=pre_run
                )
                user_input = user_input.strip()
                
                if not user_input:
                    continue
                
                if user_input.startswith("/"):
                    if await execute_command(user_input, self.ctx):
                        self._initialize_provider()
                        if self.state.session_messages:
                            self.messages = self.state.session_messages
                        continue
                    else:
                        print_error(f"Unknown command: {user_input}")
                        continue
                
                await self.process_agent_request(user_input)
                
            except KeyboardInterrupt:
                continue
            except EOFError:
                console.print("\n[info]Exiting...[/info]")
                break
            except Exception as e:
                import traceback
                print_error(f"An unexpected error occurred: {e}")
                console.print(f" [dim]{traceback.format_exc()[-300:]}[/dim]")

    async def run_single(self, query: str):
        """Run a single query in headless mode and print the result."""
        from .ui import apply_theme
        apply_theme(self.ctx.config.theme)
        self._set_terminal_background()
        self.state.start_time = time.time()
        self.state.session_messages = []
        if not self._initialize_provider():
            print_error(f"No API key found for provider '{self.ctx.config.provider}'.")
            return
        system_content = compute_system_prompt(self.ctx)
        self.messages = [{"role": "system", "content": system_content}]
        await self.process_agent_request(query)

    async def process_agent_request(self, user_input: str):
        if not self.orchestrator.provider:
            if not self.orchestrator._initialize_provider():
                print_error(f"Provider not initialized. Set your API key with [command]/config[/command]")
                return

        self.messages.append({"role": "user", "content": user_input})
        self.state.session_messages = self.messages[:]
        
        class REPLHooks(OrchestratorHooks):
            def __init__(self, repl):
                self.repl = repl
                self.live = None
                self.renderable = None
                self.pending_tool_labels = []
                self.start_time = None
                self.full_response = ""

            async def on_chunk(self, chunk):
                if chunk["type"] == "text":
                    self.full_response += chunk["content"]
                    display_text = self.full_response.replace("<thought>", "").replace("</thought>", "")
                    self.renderable.markdown = Markdown(display_text)
                    self.live.update(self.renderable)
                elif chunk["type"] == "tool_call":
                    tc = chunk["tool_call"]
                    fn = tc.get("function", {})
                    try:
                        args = json.loads(fn.get("arguments", "{}")) if isinstance(fn.get("arguments"), str) else fn.get("arguments", {})
                    except Exception:
                        args = {}
                    label = get_tool_label(fn.get("name", "?"), args)
                    if label not in self.pending_tool_labels:
                        self.pending_tool_labels.append(label)
                    self.renderable.info = "Pending: " + ", ".join(self.pending_tool_labels)
                    self.live.update(self.renderable)

            async def on_error(self, message):
                print_error(message)

            async def on_tool_call(self, name, args):
                pass

            async def on_tool_result(self, name, result, elapsed):
                if name == "file_edit" and result.get("success") and result.get("diff"):
                    from .ui import print_diff
                    print_diff(result["diff"])

            async def on_turn_complete(self, full_response, tool_calls):
                elapsed = time.time() - self.start_time
                final_response = full_response
                if "<thought>" in full_response and "</thought>" in full_response:
                    parts = full_response.split("<thought>", 1)
                    thought_parts = parts[1].split("</thought>", 1)
                    final_response = thought_parts[1].strip()
                    print_thought_elapsed(elapsed)
                    if final_response:
                        self.repl.ctx.console.print(Markdown(final_response))
                elif full_response:
                    self.repl.ctx.console.print(Markdown(full_response))
                
                print_status_line(self.repl.ctx.config.model, elapsed)
                print_session_stats(self.repl.ctx.state)

                self.repl.auto_save_counter += 1
                bus.emit("session.turn_complete", count=self.repl.auto_save_counter)
                
                if full_response:
                    pct = self.repl.ctx.state.add_tokens(count_tokens(full_response, self.repl.ctx.config.model), self.repl.ctx.config.model)
                    if pct:
                        self.repl.ctx.console.print(f" [warning]⚠[/warning] [dim]Context ~{pct:.0f}% full.[/dim]")
                        if pct > 70 and pct <= 85:
                            if self.repl._microcompact():
                                self.repl.ctx.console.print(" [success]✔[/success] [dim]Cleaned old tool results.[/dim]")
                        elif pct > 85:
                            from .ui import LoomDialog
                            options = []
                            if self.repl._microcompact():
                                options.append(("Micro (clean results)", "micro"))
                            options.append(("Full (summarize)", "full"))
                            options.append(("Continue", "continue"))
                            dialog = LoomDialog(
                                title="Context Full",
                                text=f"Context is {pct:.0f}% full. Choose action:",
                                buttons=options
                            )
                            result = await dialog.run_async()
                            if result == "full" and self.repl._compact_context():
                                self.repl.ctx.console.print(" [success]✔[/success] [dim]Context compacted.[/dim]")
                            elif result == "micro":
                                self.repl.ctx.console.print(" [success]✔[/success] [dim]Old tool results cleared.[/dim]")

            async def should_stop(self):
                return False

        hooks = REPLHooks(self)
        
        async def tool_executor(name, args):
            label = get_tool_label(name, args)
            with self.ctx.console.status(f"[dim]{label}[/dim]", spinner="dots"):
                ts = time.time()
                result = await self.call_tool(name, args)
            elapsed = time.time() - ts
            print_tool_call(name, args)
            print_tool_result(result, elapsed, name)
            return result

        try:
            progress = get_thinking_indicator()
            hooks.renderable = LoadingRenderable(progress)
            with Live(hooks.renderable, refresh_per_second=10, console=self.ctx.console, transient=True) as live:
                hooks.live = live
                hooks.start_time = time.time()
                await self.orchestrator.run(self.messages, hooks=hooks, tool_executor=tool_executor)
            
            self.state.session_messages = self.messages[:]

        except KeyboardInterrupt:
            self.ctx.console.print("\n [dim]Aborted.[/dim]")
        except Exception as e:
            import traceback
            from .errors import classify_exception
            ce = classify_exception(e)
            print_error(f"{ce.message}")
            self.ctx.console.print(f" [dim]{traceback.format_exc()[-300:]}[/dim]")
            self.messages.append(ce.to_message())

    def _microcompact(self) -> bool:
        """Strip old tool results without an API call while preserving critical context."""
        if len(self.messages) < 4:
            return False
            
        system_msg = self.messages[0]
        
        turns = []
        current_turn = []
        last_role = None
        for msg in self.messages[1:]:
            role = msg.get("role")
            if role == "user":
                if current_turn: turns.append(current_turn)
                current_turn = [msg]
            elif role == "assistant":
                if last_role in ("tool", "assistant"):
                    if current_turn: turns.append(current_turn)
                    current_turn = [msg]
                else:
                    current_turn.append(msg)
            else:
                current_turn.append(msg)
            last_role = role
            
        if current_turn:
            turns.append(current_turn)

        kept = [system_msg]
        for i, turn in enumerate(turns):
            is_recent = i >= len(turns) - 4
            has_critical_context = False
            for msg in turn:
                if msg.get("role") == "tool" and msg.get("name") in ("file_read", "file_edit", "file_write", "task"):
                    has_critical_context = True
                    break
                    
            if is_recent or has_critical_context:
                kept.extend(turn)

        if len(kept) < len(self.messages):
            self.messages = kept
            self.state.tokens_used = 0
            self.state.context_warned = False
            return True
        return False

    def _compact_context(self) -> bool:
        if len(self.messages) < 4:
            return False
        system = self.messages[0]
        keep_count = 7
        to_compact = self.messages[1:-keep_count] if len(self.messages) > keep_count + 1 else []
        if len(to_compact) < 2:
            return False

        summary_text = "\n".join(
            f"[{m.get('role','?')}]: {(m.get('content','') or '')[:200]}"
            for m in to_compact
        )

        prev_summary = ""
        for m in reversed(self.messages):
            if m.get("role") == "system" and "[Compacted]" in str(m.get("content", "")):
                prev_summary = str(m.get("content", ""))
                break

        anchor = f"\nPrevious summary (update this):\n{prev_summary}" if prev_summary else ""

        compact_prompt = (
            "Summarize this conversation. Format as Markdown headers (Goal, Constraints, Progress, Decisions, Next Steps, Critical Context, Relevant Files).\n"
            f"{anchor}\n\n"
            "Conversation to summarize:\n" + summary_text
        )

        try:
            # Note: Context compaction still uses synchronous sub-agent execution for simplicity during background compaction
            from .tools.task import _run_sub_agent
            from .task_manager import task_manager
            task_id = f"c{abs(hash(compact_prompt)) % 10**7}"
            task_manager.create("Context compaction", None, task_id)
            
            # Since we are in an async method, we can't easily call a sync function that might block.
            # But here we want to wait for it.
            import asyncio
            asyncio.create_task(asyncio.to_thread(_run_sub_agent, compact_prompt, 3, task_id, self.ctx))
            
            # We don't wait for it here to avoid blocking the turn.
            # Compaction usually happens in the background or between turns.
            return True
        except Exception:
            pass
        return False

    def _get_git_context(self) -> str:
        from .git import get_git_context
        return get_git_context()

    async def _confirm_destructive(self, tool: BaseTool, args: dict) -> bool:
        if not tool.isDestructive:
            return True

        if self._check_permission_allow(tool, args):
            return True
        if self._check_permission_deny(tool, args):
            self.ctx.console.print(f" [error]✘[/error] [dim]Blocked by permission rules: {tool.name}[/dim]")
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
                self.ctx.console.print(f" [success]✔[/success] [dim]Added {pattern} to allowlist.[/dim]")
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
            result = await asyncio.to_thread(tool.execute, **arguments, ctx=self.ctx)
            registry.run_post_hooks(name, result)
            return result
        except Exception as e:
            result = {"error": str(e)}
            registry.run_post_hooks(name, result)
            return result
