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
    print_error, print_welcome_screen, 
    print_thought_elapsed, print_status_line,
    print_tool_call, print_tool_result, print_session_stats,
    get_thinking_indicator, LoadingRenderable, get_tool_label,
    LoomFace, LOOM_FACES, print_step, get_theme_bg
)
from .commands import execute_command, get_command_metadata
from .tools import registry
from .tools.base import BaseTool
from .config import config, CONFIG_DIR
from .agents.openrouter import OpenRouterProvider
from .agents.openai import OpenAIProvider
from .agents.anthropic import AnthropicProvider
from .agents.google import GoogleProvider
from .agents.deepseek import DeepSeekProvider
from .agents.opencode_go import OpenCodeGoProvider
from .agents.opencode_zen import OpenCodeZenProvider
from .state import state, count_tokens
from .system_prompt import compute_system_prompt
from .errors import classify_exception

PROVIDER_MAP = {
    "openrouter": OpenRouterProvider,
    "openai": OpenAIProvider,
    "anthropic": AnthropicProvider,
    "google": GoogleProvider,
    "deepseek": DeepSeekProvider,
    "opencode-go": OpenCodeGoProvider,
    "opencode": OpenCodeZenProvider,
}

class _ConsoleProxy:
    """Proxy that always delegates to _ui.console, even after apply_theme reassigns it."""
    def __getattr__(self, name):
        return getattr(_ui.console, name)
    def __enter__(self):
        return _ui.console.__enter__()
    def __exit__(self, *args):
        return _ui.console.__exit__(*args)

console = _ConsoleProxy()





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
        self.auto_save_counter = 0
        self.logo_animation_count = 0

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
            ("class:toolbar", f" Build \u00b7 {config.model} "),
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
            return False
        
        provider_class = PROVIDER_MAP.get(config.provider)
        if provider_class:
            self.provider = provider_class(api_key)
            return True
        return False

    def _set_terminal_background(self):
        """Update prompt_toolkit style to match the active theme background."""
        bg = get_theme_bg()
        # Derive a slightly lighter toolbar background from the theme bg
        try:
            r = int(bg[1:3], 16)
            g = int(bg[3:5], 16)
            b = int(bg[5:7], 16)
            # Lighten each channel by ~12 for subtle contrast
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

    def run(self):
        from .ui import apply_theme
        apply_theme(config.theme)
        self._set_terminal_background()
        state.start_time = time.time()
        state.session_messages = []
        import sys
        sys.stdout.write("\033[2J\033[H")
        sys.stdout.flush()
        
        print_welcome_screen(getpass.getuser(), config.model, config.provider)
        
        if not self._initialize_provider():
            print_error(f"No API key found for provider '{config.provider}'. Use [command]/config[/command] to set it.")

        system_content = compute_system_prompt()
        self.messages = [{"role": "system", "content": system_content}]

        while True:
            try:
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

                user_input = self.session.prompt(
                    [("class:accent", "┃ "), ("class:prompt", "> ")],
                    pre_run=pre_run
                ).strip()
                
                if not user_input:
                    continue
                
                if user_input.startswith("/"):
                    if execute_command(user_input):
                        self._initialize_provider()
                        if state.session_messages:
                            self.messages = state.session_messages
                        continue
                    else:
                        print_error(f"Unknown command: {user_input}")
                        continue
                
                self.process_agent_request(user_input)
                
            except KeyboardInterrupt:
                continue
            except EOFError:
                console.print("\n[info]Exiting...[/info]")
                break
            except Exception as e:
                import traceback
                print_error(f"An unexpected error occurred: {e}")
                console.print(f" [dim]{traceback.format_exc()[-300:]}[/dim]")

    def run_single(self, query: str):
        """Run a single query in headless mode and print the result."""
        from .ui import apply_theme
        apply_theme(config.theme)
        self._set_terminal_background()
        state.start_time = time.time()
        state.session_messages = []
        if not self._initialize_provider():
            print_error(f"No API key found for provider '{config.provider}'.")
            return
        system_content = compute_system_prompt()
        self.messages = [{"role": "system", "content": system_content}]
        self.process_agent_request(query)

    def process_agent_request(self, user_input: str):
        if not self.provider:
            if not self._initialize_provider():
                print_error(f"Provider not initialized. Set your API key with [command]/config[/command]")
                return

        self.messages.append({"role": "user", "content": user_input})
        state.session_messages = self.messages[:]
        
        try:
            tool_schemas = [tool.to_json_schema() for tool in registry._tools.values()]
        except Exception as e:
            import traceback
            print_error(f"Failed to build tool schemas: {e}")
            console.print(f" [dim]{traceback.format_exc()[-500:]}[/dim]")
            return

        while True:
            start_time = time.time()
            
            full_response = ""
            tool_calls = []
            
            try:
                progress = get_thinking_indicator()
                renderable = LoadingRenderable(progress)
                with Live(renderable, refresh_per_second=10, console=console, transient=True) as live:
                    pending_tool_labels = []
                    for chunk in self.provider.ask(self.messages, config.model, tools=tool_schemas):
                        if chunk["type"] == "text":
                            full_response += chunk["content"]
                            display_text = full_response.replace("<thought>", "").replace("</thought>", "")
                            renderable.markdown = Markdown(display_text)
                            live.update(renderable)
                        
                        elif chunk["type"] == "tool_call":
                            tool_calls.append(chunk["tool_call"])
                            from .ui import get_tool_label
                            tc = chunk["tool_call"]
                            fn = tc.get("function", {})
                            try:
                                args = json.loads(fn.get("arguments", "{}")) if isinstance(fn.get("arguments"), str) else fn.get("arguments", {})
                            except Exception:
                                args = {}
                            label = get_tool_label(fn.get("name", "?"), args)
                            if label not in pending_tool_labels:
                                pending_tool_labels.append(label)
                            renderable.info = "Pending: " + ", ".join(pending_tool_labels)
                            live.update(renderable)
                        elif chunk["type"] == "error":
                            print_error(chunk["content"])
                            return

                elapsed = time.time() - start_time
                final_response = full_response
                if "<thought>" in full_response and "</thought>" in full_response:
                    parts = full_response.split("<thought>", 1)
                    thought_parts = parts[1].split("</thought>", 1)
                    final_response = thought_parts[1].strip()
                    print_thought_elapsed(elapsed)
                    if final_response:
                        console.print(Markdown(final_response))
                elif full_response:
                    console.print(Markdown(full_response))
                
                print_status_line(config.model, elapsed)
                print_session_stats()

                assistant_message = {"role": "assistant"}
                if full_response:
                    assistant_message["content"] = full_response
                if tool_calls:
                    sanitized = []
                    for tc in tool_calls:
                        t = dict(tc)
                        fn = dict(t.get("function", {}))
                        if isinstance(fn.get("arguments"), dict):
                            fn["arguments"] = json.dumps(fn["arguments"])
                        t["function"] = fn
                        sanitized.append(t)
                    assistant_message["tool_calls"] = sanitized
                
                self.messages.append(assistant_message)
                state.session_messages = self.messages[:]
                self.auto_save_counter += 1
                if self.auto_save_counter % 5 == 0:
                    from .commands import handle_save
                    handle_save(["auto"])
                
                if full_response:
                    pct = state.add_tokens(count_tokens(full_response, config.model), config.model)
                    if pct:
                        console.print(f" [warning]⚠[/warning] [dim]Context ~{pct:.0f}% full.[/dim]")
                        if pct > 70 and pct <= 85:
                            if self._microcompact():
                                console.print(" [success]✔[/success] [dim]Cleaned old tool results.[/dim]")
                            else:
                                console.print(" [dim]Consider /clear if performance degrades.[/dim]")
                        elif pct > 85:
                            from prompt_toolkit.shortcuts import button_dialog
                            options = []
                            if self._microcompact():
                                options.append(("Micro (clean results)", "micro"))
                            options.append(("Full (summarize)", "full"))
                            options.append(("Continue", "continue"))
                            result = button_dialog(
                                title="Context Full",
                                text=f"Context is {pct:.0f}% full. Choose action:",
                                buttons=options
                            ).run()
                            if result == "full" and self._compact_context():
                                console.print(" [success]✔[/success] [dim]Context compacted.[/dim]")
                            elif result == "micro":
                                console.print(" [success]✔[/success] [dim]Old tool results cleared.[/dim]")

                if not tool_calls:
                    break

                tool_inputs = []
                for tc in tool_calls:
                    tc_id = tc.get("id")
                    func = tc.get("function", {})
                    name = func.get("name")
                    if not tc_id or not name:
                        continue

                    raw = func.get("arguments", "{}")
                    if isinstance(raw, dict):
                        args = raw
                    else:
                        try:
                            args = json.loads(raw)
                        except json.JSONDecodeError as e:
                            print_error(f"Failed to parse arguments for {name}: {e}")
                            continue

                    state.tools_called += 1
                    tool_inputs.append((tc_id, name, args, tc))

                # Partition into concurrent-safe batches
                batches = self._partition_tools(tool_inputs)
                for batch in batches:
                    is_safe, items = batch

                    if is_safe and len(items) > 1:
                        for _, name, args, _ in items:
                            print_tool_call(name, args)
                        with console.status("[dim]Executing tools...[/dim]", spinner="dots"):
                            results = []
                            with concurrent.futures.ThreadPoolExecutor(max_workers=len(items)) as executor:
                                futures = {}
                                for tc_id, name, args, tc in items:
                                    future = executor.submit(self.call_tool, name, args)
                                    futures[future] = (tc_id, name)
                                for future in concurrent.futures.as_completed(futures):
                                    tc_id, name = futures[future]
                                    result = future.result()
                                    results.append((tc_id, name, result))
                        for tc_id, name, result in results:
                            self._append_tool_result(tc_id, name, result)
                        print_step(f"Completed {len(items)} tool(s)")
                    else:
                        for tc_id, name, args, tc in (items if isinstance(items, list) else [(None,) + items]):
                            label = get_tool_label(name, args)
                            with console.status(f"[dim]{label}[/dim]", spinner="dots"):
                                start_ts = time.time()
                                result = self.call_tool(name, args)
                            elapsed_ts = time.time() - start_ts
                            print_tool_call(name, args)
                            print_tool_result(result, elapsed_ts, name)
                            self._append_tool_result(tc_id, name, result)

            except KeyboardInterrupt:
                console.print("\n [dim]Aborted.[/dim]")
                break
            except Exception as e:
                import traceback
                ce = classify_exception(e)
                print_error(f"{ce.message}")
                console.print(f" [dim]{traceback.format_exc()[-300:]}[/dim]")
                self.messages.append(ce.to_message())
                if not ce.recoverable:
                    console.print(f" [dim]{ce.guidance}[/dim]")
                break

    def _microcompact(self) -> bool:
        """Strip old tool results without an API call."""
        if len(self.messages) < 4:
            return False
        # Keep system prompt + last 3 user/assistant turns + their tool results
        kept = [self.messages[0]]
        tool_ids_to_keep = set()
        turn_count = 0
        for msg in reversed(self.messages[1:]):
            if msg.get("role") == "tool":
                if msg.get("tool_call_id") in tool_ids_to_keep:
                    kept.insert(1, msg)
            else:
                kept.insert(1, msg)
                if msg.get("role") in ("user", "assistant"):
                    turn_count += 1
                    if msg.get("tool_calls"):
                        for tc in (msg.get("tool_calls") or []):
                            tool_ids_to_keep.add(tc.get("id", ""))
                if turn_count >= 3:
                    break
        # Add remaining tool messages that belong to kept turns
        for msg in reversed(self.messages[1:]):
            if msg.get("role") == "tool" and msg.get("tool_call_id") in tool_ids_to_keep:
                if msg not in kept:
                    kept.insert(1, msg)

        if len(kept) < len(self.messages):
            self.messages = kept
            state.tokens_used = 0
            state.context_warned = False
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
            "Summarize this conversation using the format below. Keep every section even if empty.\n\n"
            "## Goal\n[single-sentence task summary]\n\n"
            "## Constraints & Preferences\n[user constraints]\n\n"
            "## Progress\n### Done | In Progress | Blocked\n\n"
            "## Key Decisions\n\n"
            "## Next Steps\n\n"
            "## Critical Context\n[important context to preserve]\n\n"
            "## Relevant Files\n[file paths that were read, edited, or created]\n\n"
            "Use terse bullets. Preserve exact paths, commands, and errors.\n"
            f"{anchor}\n\n"
            "Conversation to summarize:\n" + summary_text
        )

        try:
            from .tools.task import _run_sub_agent
            from .task_manager import task_manager
            task_id = f"c{abs(hash(compact_prompt)) % 10**7}"
            task_manager.create("Context compaction", None, task_id)
            _run_sub_agent(compact_prompt, 3, task_id)
            record = task_manager.get(task_id)
            summary = record.result.get("output", "") if record and record.result else ""
            if summary:
                self.messages = [system, {"role": "system", "content": f"[Compacted summary]\n{summary[:3000]}"}]
                keep = self.messages[-keep_count:] if len(self.messages) > keep_count else []
                self.messages = self.messages[:2] + keep
                state.tokens_used = 0
                state.context_warned = False
                return True
        except Exception:
            pass
        return False

    def _get_git_context(self) -> str:
        try:
            import subprocess
            status = subprocess.run("git status --short", shell=True, capture_output=True, text=True, timeout=5).stdout.strip()
            log = subprocess.run("git log --oneline -5", shell=True, capture_output=True, text=True, timeout=5).stdout.strip()
            branch = subprocess.run("git rev-parse --abbrev-ref HEAD", shell=True, capture_output=True, text=True, timeout=5).stdout.strip()
            parts = []
            if branch:
                parts.append(f"Current branch: {branch}")
            if status:
                lines = status.split("\n")[:20]
                parts.append(f"Changed files ({len(lines)}):\n" + "\n".join(lines))
            if log:
                parts.append(f"Recent commits:\n{log[:500]}")
            return "## Git Context\n" + "\n".join(parts) if parts else ""
        except Exception:
            return ""

    def _partition_tools(self, tool_inputs: list) -> list:
        batches = []
        current_batch = []
        for item in tool_inputs:
            name = item[1]
            tool = registry.get_tool(name)
            is_safe = tool.isConcurrencySafe if tool else False
            if is_safe:
                current_batch.append(item)
            else:
                if current_batch:
                    batches.append((True, current_batch))
                    current_batch = []
                batches.append((False, [item]))
        if current_batch:
            batches.append((True, current_batch))
        return batches

    def _append_tool_result(self, tc_id: str, name: str, result: dict):
        from .ui import print_diff
        MAX_CHARS = 50000
        content = json.dumps(result)
        if len(content) > MAX_CHARS:
            path = CONFIG_DIR / "tool_results" / f"{tc_id}.json"
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content, encoding="utf-8")
            result["content"] = f"[Result too large, saved to {path}]\n{result.get('content', '')[:2000]}"
            content = json.dumps(result)

        self.messages.append({
            "role": "tool", "tool_call_id": tc_id,
            "name": name, "content": content
        })
        state.add_tokens(count_tokens(content, config.model), config.model)

        if name == "file_read" and result.get("success") and result.get("content"):
            console.print(Markdown(result["content"]))
        if name == "file_edit" and result.get("success") and result.get("diff"):
            print_diff(result["diff"])

    def _confirm_destructive(self, tool: BaseTool, args: dict) -> bool:
        if not tool.isDestructive:
            return True

        if self._check_permission_allow(tool, args):
            return True
        if self._check_permission_deny(tool, args):
            console.print(f" [error]✘[/error] [dim]Blocked by permission rules: {tool.name}[/dim]")
            return False

        from .ui import get_tool_label, print_diff
        import difflib
        label = get_tool_label(tool.name, args)

        if tool.name == "file_edit":
            old_str = args.get("old_string", "")
            new_str = args.get("new_string", "")
            if old_str and new_str:
                old_lines = old_str.splitlines(keepends=True)
                new_lines = new_str.splitlines(keepends=True)
                diff_lines = list(difflib.unified_diff(
                    old_lines, new_lines,
                    fromfile=args.get("file_path", "a"), tofile=args.get("file_path", "b"),
                    lineterm=''
                ))
                print_diff(diff_lines)

        try:
            from prompt_toolkit.shortcuts import button_dialog
            result = button_dialog(
                title="Confirm Destructive Action",
                text=f"Allow {label}?",
                buttons=[("Allow", True), ("Deny", True), ("Deny", False)]
            ).run()
            return result if result is not None else False
        except Exception:
            return True

    def _check_permission_allow(self, tool: BaseTool, args: dict) -> bool:
        from .config import config
        from fnmatch import fnmatch
        allowlist = config.allowlist or []
        pattern = f"{tool.name}(*)"
        for rule in allowlist:
            if fnmatch(pattern, rule):
                return True
        return False

    def _check_permission_deny(self, tool: BaseTool, args: dict) -> bool:
        from .config import config
        from fnmatch import fnmatch
        denylist = config.denylist or []
        pattern = f"{tool.name}(*)"
        for rule in denylist:
            if fnmatch(pattern, rule):
                return True
        return False

    def call_tool(self, name: str, arguments: dict):
        tool = registry.get_tool(name)
        if not tool:
            return {"error": f"Tool not found: {name}"}

        registry.run_pre_hooks(name, arguments)
        if tool.isDestructive and not self._confirm_destructive(tool, arguments):
            registry.run_post_hooks(name, {"error": "Permission denied"})
            return {"error": "Permission denied by user"}

        try:
            result = tool.execute(**arguments)
            registry.run_post_hooks(name, result)
            return result
        except Exception as e:
            result = {"error": str(e)}
            registry.run_post_hooks(name, result)
            return result
