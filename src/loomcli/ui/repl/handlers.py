import time
import asyncio
import re
from ...core.orchestrator import OrchestratorHooks
from rich.markup import escape
from .. import (
    print_status_line,
    print_tool_call,
    print_tool_result,
    print_session_stats,
    print_diff,
)


def format_duration(seconds: float) -> str:
    if seconds < 1:
        return f"{seconds:.1f}s"
    seconds = int(seconds)
    if seconds < 60:
        return f"{seconds}s"
    m = seconds // 60
    s = seconds % 60
    return f"{m}m {s}s"

class AppHooks(OrchestratorHooks):
    def __init__(self, repl):
        self.repl = repl
        self.full_response = ""
        self._stream_buffer = ""
        self._in_thought = False
        self._thought_start_time = 0
        self._last_header_ansi = ""
        self._last_header_ansi = ""
        
        from rich.console import Console
        self._dummy_console = Console(force_terminal=True, color_system="truecolor")
        
        self._typing_cursor = " █"
        self._thinking_text = "Thinking..."
        self._is_thinking = False
        self._first_chunk = True
        self.start_time = 0
        self._text_start_pos = None
        self._thought_durations = []

    def _remove_cursor(self):
        text = self.repl.history_buffer.text
        if text.endswith(self._typing_cursor):
            old_cursor = self.repl.history_buffer.cursor_position
            self.repl.history_buffer.text = text[: -len(self._typing_cursor)]
            if old_cursor > len(self.repl.history_buffer.text):
                old_cursor = len(self.repl.history_buffer.text)
            self.repl.history_buffer.cursor_position = old_cursor

    def _add_thinking(self):
        self._is_thinking = True
        was_at_bottom = self.repl._is_scrolled_to_bottom()
        old_cursor = self.repl.history_buffer.cursor_position
        self.repl.history_buffer.cursor_position = len(self.repl.history_buffer.text)
        self.repl.history_buffer.insert_text(self._thinking_text)
        if not was_at_bottom:
            self.repl.history_buffer.cursor_position = old_cursor
        if self.repl.app:
            self.repl.app.invalidate()

    def _remove_thinking(self):
        if not self._is_thinking:
            return

        text = self.repl.history_buffer.text
        old_cursor = self.repl.history_buffer.cursor_position
        # Remove thinking text and trailing whitespace
        self.repl.history_buffer.text = re.sub(
            re.escape(self._thinking_text) + r"\s*$", "", text
        )
        if self.repl._is_scrolled_to_bottom():
            self.repl.history_buffer.cursor_position = len(self.repl.history_buffer.text)
        else:
            if old_cursor > len(self.repl.history_buffer.text):
                old_cursor = len(self.repl.history_buffer.text)
            self.repl.history_buffer.cursor_position = old_cursor
        self._is_thinking = False

    async def _start_thought(self):
        if self._in_thought:
            return
        self._in_thought = True
        self._thought_start_time = time.time()
        
        header_text = "Thinking:"
        full_markup = f"\n[dim]│[/dim] [italic #ffaf00]{header_text}[/italic #ffaf00] "
        
        self._dummy_console.width = self.repl._rich_console.width
        with self._dummy_console.capture() as capture:
            self._dummy_console.print(full_markup, end="")
        self._last_header_ansi = capture.get()

        self.repl._rich_console.print(full_markup, end="")



    def _end_thought(self):
        if not self._in_thought:
            return
        self._in_thought = False
            
        duration = time.time() - self._thought_start_time
        dur_str = format_duration(duration)
        self._thought_durations.append(dur_str)
        # We don't use ":" here per user request for final state
        self.repl._rich_console.print(f"\n[dim]│[/dim] [dim italic]Thought for {dur_str}[/dim italic]\n\n", end="")

    def _add_cursor(self):
        was_at_bottom = self.repl._is_scrolled_to_bottom()
        old_cursor = self.repl.history_buffer.cursor_position
        self.repl.history_buffer.cursor_position = len(self.repl.history_buffer.text)
        self.repl.history_buffer.insert_text(self._typing_cursor)
        if not was_at_bottom:
            self.repl.history_buffer.cursor_position = old_cursor

    async def on_chunk(self, chunk):
        if self._first_chunk and chunk["type"] in ("text", "reasoning", "tool_call"):
            self._remove_thinking()
            self._first_chunk = False

        if chunk["type"] == "reasoning":
            self._remove_cursor()
            await self._start_thought()
            content = chunk["content"]
            formatted = escape(content).replace("\n", "\n[dim]│[/dim] ")
            self.repl._rich_console.print(f"[dim italic]{formatted}[/dim italic]", end="")
            self._add_cursor()
            return

        if chunk["type"] == "text":
            if self._in_thought and not self._stream_buffer:
                # Transitioning from native reasoning chunks to text chunks
                self._remove_cursor()
                self._end_thought()
                self._add_cursor()

            if self._text_start_pos is None:
                self._remove_cursor()
                self._text_start_pos = len(self.repl.history_buffer.text)
                self._add_cursor()

            self.full_response += chunk["content"]
            self._stream_buffer += chunk["content"]
            self._remove_cursor()

            while True:
                if not self._in_thought:
                    if "<thought>" in self._stream_buffer:
                        parts = self._stream_buffer.split("<thought>", 1)
                        if parts[0]:
                            self.repl._rich_console.print(parts[0], end="", markup=False)
                        await self._start_thought()
                        self._stream_buffer = parts[1]
                        continue
                    else:
                        safe_len = len(self._stream_buffer)
                        for i in range(1, len("<thought>")):
                            if self._stream_buffer.endswith("<thought>"[:i]):
                                safe_len -= i
                                break
                        if safe_len > 0:
                            self._stream_buffer = self._stream_buffer[safe_len:]
                            
                            # Live Markdown rendering!
                            # We truncate back to the start of the text response and re-render everything
                            if self._text_start_pos is not None:
                                from ..renderables import EnhancedMarkdown as Markdown
                                from rich.markdown import Markdown as RichMarkdown
                                
                                was_at_bottom = self.repl._is_scrolled_to_bottom()
                                old_cursor = self.repl.history_buffer.cursor_position
                                
                                # Temporarily remove current text to re-render
                                self.repl.history_buffer.text = self.repl.history_buffer.text[: self._text_start_pos]
                                
                                # Use a temporary console to get ANSI output
                                with self._dummy_console.capture() as capture:
                                    # Split by <thought> tags to render parts correctly
                                    parts = re.split(r"(<thought>.*?</thought>|<thought>.*$)", self.full_response, flags=re.DOTALL)
                                    thought_idx = 0
                                    for part in parts:
                                        if part.startswith("<thought>"):
                                            content = part[len("<thought>"):]
                                            if content.endswith("</thought>"):
                                                content = content[:-len("</thought>")]
                                            
                                            dur_str = self._thought_durations[thought_idx] if thought_idx < len(self._thought_durations) else "..."
                                            thought_idx += 1
                                            
                                            self._dummy_console.print(f"\n[dim]│[/dim] [italic #ffaf00]Thought for {dur_str}:[/italic #ffaf00] ")
                                            formatted = escape(content.strip()).replace("\n", "\n[dim]│[/dim] ")
                                            self._dummy_console.print(f"[dim italic]│ {formatted}[/dim italic]\n")
                                        else:
                                            if part.strip():
                                                self._dummy_console.print(Markdown(part))
                                
                                from rich.text import Text
                                self.repl._rich_console.print(Text.from_ansi(capture.get()), end="")
                                
                                if was_at_bottom:
                                    self.repl.history_buffer.cursor_position = len(self.repl.history_buffer.text)
                                elif old_cursor > len(self.repl.history_buffer.text):
                                    self.repl.history_buffer.cursor_position = len(self.repl.history_buffer.text)
                                else:
                                    self.repl.history_buffer.cursor_position = old_cursor
                        break
                else:
                    if "</thought>" in self._stream_buffer:
                        parts = self._stream_buffer.split("</thought>", 1)
                        if parts[0]:
                            formatted = escape(parts[0]).replace("\n", "\n[dim]│[/dim] ")
                            self.repl._rich_console.print(
                                f"[dim italic]{formatted}[/dim italic]", end=""
                            )
                        self._end_thought()
                        self._stream_buffer = parts[1]
                        continue
                    else:
                        safe_len = len(self._stream_buffer)
                        for i in range(1, len("</thought>")):
                            if self._stream_buffer.endswith("</thought>"[:i]):
                                safe_len -= i
                                break
                        if safe_len > 0:
                            formatted = escape(self._stream_buffer[:safe_len]).replace(
                                "\n", "\n[dim]│[/dim] "
                            )
                            self.repl._rich_console.print(f"[dim italic]{formatted}[/dim italic]", end="")
                            self._stream_buffer = self._stream_buffer[safe_len:]
                        break

            self._add_cursor()

    async def on_turn_complete(self, full_response, tool_calls):
        self._remove_cursor()
        self._remove_thinking()

        if self._in_thought:
            if self._stream_buffer:
                formatted = escape(self._stream_buffer).replace("\n", "\n[dim]│[/dim] ")
                self.repl._rich_console.print(f"[dim italic]{formatted}[/dim italic]", end="")
            self._end_thought()
        else:
            if self._stream_buffer:
                self.repl._rich_console.print(escape(self._stream_buffer), end="", markup=False)

        self._stream_buffer = ""

        # Re-render the entire text response as Markdown
        if self._text_start_pos is not None and self.full_response.strip():
            was_at_bottom = self.repl._is_scrolled_to_bottom()
            old_cursor = self.repl.history_buffer.cursor_position
            self.repl.history_buffer.text = self.repl.history_buffer.text[: self._text_start_pos]
            if was_at_bottom:
                self.repl.history_buffer.cursor_position = len(self.repl.history_buffer.text)
            elif old_cursor > len(self.repl.history_buffer.text):
                self.repl.history_buffer.cursor_position = len(self.repl.history_buffer.text)
            else:
                self.repl.history_buffer.cursor_position = old_cursor

            from ..renderables import EnhancedMarkdown as Markdown
            from rich.markdown import Markdown as RichMarkdown

            # Split by <thought> tags
            parts = re.split(r"(<thought>.*?</thought>)", self.full_response, flags=re.DOTALL)
            thought_idx = 0
            for part in parts:
                if part.startswith("<thought>") and part.endswith("</thought>"):
                    content = part[len("<thought>") : -len("</thought>")]
                    dur_str = self._thought_durations[thought_idx] if thought_idx < len(self._thought_durations) else "..."
                    thought_idx += 1
                    
                    self.repl._rich_console.print(f"\n[dim]│[/dim] [italic #ffaf00]Thought for {dur_str}:[/italic #ffaf00] ")
                    formatted = escape(content.strip()).replace("\n", "\n[dim]│[/dim] ")
                    self.repl._rich_console.print(f"[dim italic]│ {formatted}[/dim italic]\n")
                else:
                    if part.strip():
                        self.repl._rich_console.print(Markdown(part))

        if not tool_calls:
            elapsed = time.time() - self.start_time
            self.repl._rich_console.print("\n")
            print_status_line(self.repl.ctx.config.model, elapsed)
            print_session_stats(self.repl.ctx.state)

    async def on_tool_call(self, name, args):
        print_tool_call(name, args)

    async def on_tool_result(self, name, result, elapsed):
        print_tool_result(result, elapsed, name)
        if name == "file_edit" and result.get("success") and result.get("diff"):
            print_diff(result["diff"])

    async def on_error(self, message):
        self._remove_cursor()
        self._remove_thinking()
        self.repl._rich_console.print(f"\n [error]✘[/error] [error]{message}[/error]")
        self._add_cursor()
