from datetime import datetime
from typing import List
from rich.table import Table
from .. import ui as _ui
from ..ui import print_success, print_error, RouteCodeDialog
from ..core import RouteCodeContext


def handle_history(args: List[str], ctx: RouteCodeContext):
    if not ctx.state.session_messages:
        ctx.console.print("[dim]No conversation history yet.[/dim]")
        return

    table = Table(
        title="Conversation History", show_header=True, header_style="bold magenta"
    )
    table.add_column("#", style="dim")
    table.add_column("Role", style="bold")
    table.add_column("Content")

    for i, msg in enumerate(ctx.state.session_messages):
        role = msg["role"]
        content = msg.get("content", "")
        if isinstance(content, list):
            content = " ".join(
                c.get("text", "") for c in content if c.get("type") == "text"
            )
        tc = msg.get("tool_calls", [])
        if tc:
            names = [t.get("function", {}).get("name", "?") for t in tc]
            content = (content or "") + f" [dim](tool calls: {', '.join(names)})[/dim]"

        max_len = 120
        display = content[:max_len] + "..." if len(content) > max_len else content
        table.add_row(str(i), role, display.replace("[", "\\["))
    ctx.console.print(table)


async def handle_save(args: List[str], ctx: RouteCodeContext):
    has_content = any(m.get("role") != "system" for m in ctx.state.session_messages)
    if not has_content:
        if args:  # Only complain if explicitly saving, not on auto-save/exit
            print_error("No conversation to save.")
        return

    from ..core import save_session_async

    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    if args:
        name = "_".join(args).replace("/", "_")
    else:
        name = f"session_{timestamp}"

    try:
        await save_session_async(ctx.state, name)
        print_success(f"Session saved: {name}")
    except Exception as e:
        print_error(f"Failed to save session: {e}")


async def handle_load(args: List[str], ctx: RouteCodeContext):
    from ..core import SESSIONS_DIR, load_session

    if not SESSIONS_DIR.exists():
        print_error("No saved sessions found.")
        return

    session_files = sorted(SESSIONS_DIR.glob("*.json"), reverse=True)
    if not session_files:
        print_error("No saved sessions found.")
        return

    choices = []
    for sf in session_files[:20]:
        try:
            from ..utils.storage import AtomicJsonStore

            data = AtomicJsonStore(sf).load()
            label = f"{sf.stem} ({data.get('saved_at', '?')})"
            choices.append((sf.stem, label))
        except Exception:
            choices.append((sf.stem, sf.stem))

    result = await RouteCodeDialog(
        title="Load Session",
        text=_ui.get_dialog_text("Select a session to load:", "radio"),
        values=choices,
        dialog_type="radio",
    ).run_async()

    if result:
        try:
            new_state = load_session(result)
            if new_state:
                ctx.state.session_messages.set_messages(new_state.session_messages)
                ctx.state.tokens_used = new_state.tokens_used
                ctx.state.estimated_cost = new_state.estimated_cost
                ctx.state.commands_run = new_state.commands_run
                ctx.state.tools_called = new_state.tools_called
                print_success(f"Loaded session: {result}")
            else:
                print_error(f"Failed to load session: {result}")
        except Exception as e:
            print_error(f"Error loading session: {e}")


async def handle_new(args: List[str], ctx: RouteCodeContext):
    from ..core.events import bus

    ctx.state.session_messages.clear()
    ctx.state.tokens_used = 0
    ctx.state.estimated_cost = 0.0
    ctx.state.commands_run = 0
    ctx.state.tools_called = 0
    ctx.state.start_time = 0.0

    bus.emit("session.reset")
    print_success("New session started.")


def handle_rewind(args: List[str], ctx: RouteCodeContext):
    if not ctx.state.session_messages:
        print_error("No conversation to rewind.")
        return
    try:
        count = int(args[0]) if args else 1
    except ValueError:
        print_error("Usage: /rewind [count]. Count must be a number.")
        return

    turns = 0
    cut_at = len(ctx.state.session_messages)
    for i in range(len(ctx.state.session_messages) - 1, -1, -1):
        role = ctx.state.session_messages[i].get("role", "")
        if role in ("assistant", "user", "tool"):
            turns += 1
            if turns >= count:
                cut_at = i
                break

    ctx.state.session_messages.set_messages(
        ctx.state.session_messages[: max(1, cut_at)]
    )
    ctx.state.tokens_used = 0
    ctx.state.context_warned = False
    print_success(
        f"Rewound {count} turn(s). {len(ctx.state.session_messages) - 1} message(s) remaining (excluding system)."
    )


async def handle_edit(args: List[str], ctx: RouteCodeContext):
    if not ctx.state.session_messages:
        print_error("No conversation to edit.")
        return
    try:
        idx = int(args[0]) if args else -1
    except ValueError:
        print_error("Usage: /edit <message_index>. Use /history to see indices.")
        return

    if idx < 0 or idx >= len(ctx.state.session_messages):
        print_error(
            f"Index {idx} out of range (0-{len(ctx.state.session_messages) - 1}). Use /history to see indices."
        )
        return

    msg = ctx.state.session_messages[idx]
    old_content = msg.get("content", "")
    if isinstance(old_content, list):
        old_content = " ".join(
            c.get("text", "") for c in old_content if isinstance(c, dict)
        )

    new_content = await RouteCodeDialog(
        title=f"Edit Message {idx} (role: {msg['role']})",
        text=_ui.get_dialog_text("Edit the message content:", "input"),
        default=old_content,
        dialog_type="input",
    ).run_async()

    if new_content is not None and new_content != old_content:
        msg["content"] = new_content
        ctx.state.session_messages.set_messages(ctx.state.session_messages[: idx + 1])
        ctx.state.tokens_used = 0
        ctx.state.context_warned = False
        print_success(f"Message {idx} updated. Conversation truncated after edit.")
    elif new_content is not None:
        ctx.console.print("[dim]No changes made.[/dim]")


def handle_search(args: List[str], ctx: RouteCodeContext):
    if not args:
        print_error("Usage: /search <term>")
        return
    term = " ".join(args)
    if not ctx.state.session_messages:
        ctx.console.print("[dim]No conversation to search.[/dim]")
        return

    results = []
    for i, msg in enumerate(ctx.state.session_messages):
        content = msg.get("content", "")
        if isinstance(content, list):
            content = " ".join(
                c.get("text", "") for c in content if isinstance(c, dict)
            )
        if isinstance(content, str) and term.lower() in content.lower():
            results.append((i, msg["role"], content[:120]))

    if not results:
        ctx.console.print(f"[dim]No matches for '{term}'.[/dim]")
        return

    table = Table(
        title=f"Search: '{term}'", show_header=True, header_style="bold magenta"
    )
    table.add_column("#", style="dim")
    table.add_column("Role", style="bold")
    table.add_column("Content")
    for idx, role, content in results:
        table.add_row(str(idx), role, content.replace("[", "\\["))
    ctx.console.print(table)
