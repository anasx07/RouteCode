import sys
import os
import json
import time
from datetime import datetime
from typing import List, Dict
from rich.table import Table
from rich.markdown import Markdown
from prompt_toolkit.shortcuts import radiolist_dialog, input_dialog, button_dialog, message_dialog

from . import ui as _ui
from .ui import print_info, print_success, print_error, print_step, LoomDialog
from .tools import registry
from .config import config, CONFIG_DIR

PROVIDER_LIST = ["openrouter", "anthropic", "openai", "google", "deepseek", "opencode", "opencode-go"]

def handle_help(args: List[str]):
    table = Table(title="Available Commands", show_header=True, header_style="bold magenta")
    table.add_column("Command", style="command")
    table.add_column("Description")
    
    table.add_row("/help", "Show help menu")
    table.add_row("/tools", "List available agent tools")
    table.add_row("/provider", "Interactively switch AI provider")
    table.add_row("/model [name]", "Get or set the active model")
    table.add_row("/config", "Manage API keys and settings")
    table.add_row("/clear", "Clear the terminal screen")
    table.add_row("/exit", "Exit the session")
    
    _ui.console.print(table)

def handle_provider(args: List[str]):
    result = LoomDialog(
        title="Select AI Provider",
        text=_ui.get_dialog_text("Choose the provider you want to use for this session:", "radio"),
        values=[(p, p.capitalize()) for p in PROVIDER_LIST],
        dialog_type="radio"
    ).run()

    if result:
        config.provider = result
        config.save()
        _ui.console.print(f"\n[success]✔[/success] Provider switched to [bold cyan]{result}[/bold cyan]")
        
        # Check for API key
        if not config.get_api_key(result):
            new_key = LoomDialog(
                title=f"Setup {result.capitalize()}",
                text=_ui.get_dialog_text(f"No API key found for {result}. Please paste it here and press Enter:", "input"),
                password=True,
                dialog_type="input"
            ).run()
            if new_key:
                config.set_api_key(result, new_key)
                print_success(f"API key for {result} has been saved.")

def handle_model(args: List[str]):
    if args:
        config.model = args[0]
        config.save()
        _ui.console.print(f"Model set to: [bold green]{config.model}[/bold green]")
        return

    from .repl import PROVIDER_MAP
    api_key = config.get_api_key()
    if not api_key:
        print_error(f"No API key found for {config.provider}. Set it first with [command]/config[/command]")
        return

    provider_cls = PROVIDER_MAP.get(config.provider)
    if not provider_cls:
        print_error(f"Unknown provider: {config.provider}")
        return

    print_step(f"Fetching models from {config.provider}...")
    provider = provider_cls(api_key)
    models = getattr(provider, "get_models", lambda: [])()
    if models:
        model_choices = [(m["id"], m.get("name", m["id"])) for m in models]

        result = LoomDialog(
            title="Select AI Model",
            text=_ui.get_dialog_text(f"Choose a model from {config.provider}:", "radio"),
            values=model_choices,
            dialog_type="radio"
        ).run()

        if result:
            config.model = result
            config.save()
            _ui.console.print(f"\n[success]✔[/success] Model set to [bold cyan]{result}[/bold cyan]")
    else:
        _ui.console.print(f"[dim]Model listing not available for {config.provider}. "
                      f"Set the model manually: /model <name>[/dim]")

def handle_config(args: List[str]):
    # If no args, show a management menu instead of just a table
    action = LoomDialog(
        title="LoomCLI Configuration",
        text=_ui.get_dialog_text("What would you like to do?", "button"),
        buttons=[
            ("View", "view"),
            ("Update Key", "update"),
            ("Delete Key", "delete"),
            ("Back", "back")
        ],
        dialog_type="button"
    ).run()

    if action == "view":
        table = Table(title="Current Configuration", show_header=True, header_style="bold magenta")
        table.add_column("Key")
        table.add_column("Value")
        table.add_row("Provider", config.provider)
        table.add_row("Model", config.model)
        for p, key in config.api_keys.items():
            masked_key = key[:4] + "*" * (max(0, len(key) - 8)) + key[-4:] if len(key) > 8 else "****"
            table.add_row(f"{p} API Key", masked_key)
        _ui.console.print(table)

    elif action == "update":
        p_to_update = LoomDialog(
            title="Update API Key",
            text=_ui.get_dialog_text("Select which provider's key you want to update:", "radio"),
            values=[(p, p.capitalize()) for p in PROVIDER_LIST],
            dialog_type="radio"
        ).run()
        if p_to_update:
            new_key = LoomDialog(
                title=f"Update {p_to_update.capitalize()}",
                text=_ui.get_dialog_text(f"Paste your new {p_to_update} API key:", "input"),
                password=True,
                dialog_type="input"
            ).run()
            if new_key:
                config.set_api_key(p_to_update, new_key)
                print_success(f"Key for {p_to_update} has been replaced.")

    elif action == "delete":
        existing_keys = list(config.api_keys.keys())
        if not existing_keys:
            LoomDialog(title="Error", text=_ui.get_dialog_text("No API keys found to delete.", "message"), dialog_type="message").run()
            return
            
        p_to_delete = LoomDialog(
            title="Delete API Key",
            text=_ui.get_dialog_text("Select which provider's key you want to remove:", "radio"),
            values=[(p, p.capitalize()) for p in existing_keys],
            dialog_type="radio"
        ).run()
        if p_to_delete:
            confirm = LoomDialog(
                title="Confirm Deletion",
                text=_ui.get_dialog_text(f"Are you sure you want to delete the {p_to_delete} key?", "button"),
                buttons=[("Yes", True), ("No", False)],
                dialog_type="button"
            ).run()
            if confirm:
                del config.api_keys[p_to_delete]
                config.save()
                print_success(f"Key for {p_to_delete} has been deleted.")

def handle_tools(args: List[str]):
    table = Table(title="Agent Tools", show_header=True, header_style="bold cyan")
    table.add_column("Tool", style="bold yellow")
    table.add_column("Description")
    for name, desc in registry.list_tools().items():
        table.add_row(name, desc)
    _ui.console.print(table)

def handle_clear(args: List[str]):
    import sys
    sys.stdout.write("\033[2J\033[H")
    h = 24
    try:
        from prompt_toolkit.output.defaults import create_output
        h = create_output().get_size().rows
    except Exception:
        import shutil
        h = shutil.get_terminal_size().lines
    sys.stdout.write("\n" * (h - 2))
    sys.stdout.flush()

def handle_history(args: List[str], state):
    if not state.session_messages:
        _ui.console.print("[dim]No conversation history yet.[/dim]")
        return

    table = Table(title="Conversation History", show_header=True, header_style="bold magenta")
    table.add_column("#", style="dim")
    table.add_column("Role", style="bold")
    table.add_column("Content")

    for i, msg in enumerate(state.session_messages):
        role = msg["role"]
        content = msg.get("content", "")
        if isinstance(content, list):
            content = " ".join(c.get("text", "") for c in content if c.get("type") == "text")
        tc = msg.get("tool_calls", [])
        if tc:
            names = [t.get("function", {}).get("name", "?") for t in tc]
            content = (content or "") + f" [dim](tool calls: {', '.join(names)})[/dim]"

        max_len = 120
        display = content[:max_len] + "..." if len(content) > max_len else content
        table.add_row(str(i), role, display.replace("[", "\\["))
    _ui.console.print(table)

def handle_save(args: List[str], state):
    if not state.session_messages:
        print_error("No conversation to save.")
        return

    sessions_dir = CONFIG_DIR / "sessions"
    sessions_dir.mkdir(parents=True, exist_ok=True)

    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    if args:
        name = "_".join(args).replace("/", "_")
    else:
        name = f"session_{timestamp}"
    save_path = sessions_dir / f"{name}.json"

    data = {
        "saved_at": timestamp,
        "provider": config.provider,
        "model": config.model,
        "tokens_used": state.tokens_used,
        "messages": state.session_messages,
    }
    save_path.write_text(json.dumps(data, indent=2), encoding="utf-8")
    print_success(f"Session saved to {save_path}")

def handle_load(args: List[str], state):
    sessions_dir = CONFIG_DIR / "sessions"
    if not sessions_dir.exists():
        print_error("No saved sessions found.")
        return

    session_files = sorted(sessions_dir.glob("*.json"), reverse=True)
    if not session_files:
        print_error("No saved sessions found.")
        return

    choices = []
    for sf in session_files[:20]:
        try:
            data = json.loads(sf.read_text(encoding="utf-8"))
            label = f"{sf.stem} ({data.get('model', '?')}, {data.get('saved_at', '?')})"
            choices.append((str(sf), label))
        except Exception:
            choices.append((str(sf), sf.stem))

    result = LoomDialog(
        title="Load Session",
        text=_ui.get_dialog_text("Select a session to load:", "radio"),
        values=choices,
        dialog_type="radio"
    ).run()

    if result:
        try:
            data = json.loads(Path(result).read_text(encoding="utf-8"))
            state.session_messages = data.get("messages", [])
            state.tokens_used = data.get("tokens_used", 0)
            config.provider = data.get("provider", config.provider)
            config.model = data.get("model", config.model)
            print_success(f"Loaded session: {Path(result).stem}")
        except Exception as e:
            print_error(f"Failed to load session: {e}")

def handle_exit(args: List[str], state):
    handle_save([], state)
    _ui.console.print("[info]ℹ[/info] Goodbye!")
    sys.exit(0)

def handle_tasks(args: List[str]):
    from .task_manager import task_manager
    tasks = task_manager.list()
    if not tasks:
        _ui.console.print("[dim]No active or recent tasks.[/dim]")
        return
    table = Table(title="Tasks", show_header=True, header_style="bold cyan")
    table.add_column("ID", style="bold yellow")
    table.add_column("Description")
    table.add_column("Status", style="bold")
    table.add_column("Age")
    for t in tasks:
        status_style = {"running": "bold green", "completed": "dim", "failed": "bold red", "killed": "dim"}.get(t["status"], "")
        elapsed = time.time() - t["created_at"]
        age = f"{elapsed:.0f}s" if elapsed < 120 else f"{elapsed/60:.0f}m"
        table.add_row(t["task_id"], t["description"][:60], f"[{status_style}]{t['status']}[/{status_style}]", age)
    _ui.console.print(table)

def handle_task_stop(args: List[str]):
    from .task_manager import task_manager
    if not args:
        print_error("Usage: /task-stop <task_id>")
        return
    task_id = args[0]
    if task_manager.kill(task_id):
        print_success(f"Task {task_id} stopped.")
    else:
        print_error(f"Task {task_id} not found or not running.")

def handle_rewind(args: List[str], state):
    if not state.session_messages:
        print_error("No conversation to rewind.")
        return
    try:
        count = int(args[0]) if args else 1
    except ValueError:
        print_error("Usage: /rewind [count]. Count must be a number.")
        return

    # Count actual assistant+turns (skip system)
    turns = 0
    cut_at = len(state.session_messages)
    for i in range(len(state.session_messages) - 1, -1, -1):
        role = state.session_messages[i].get("role", "")
        if role in ("assistant", "user", "tool"):
            turns += 1
            if turns >= count:
                cut_at = i
                break

    kept = state.session_messages[:max(1, cut_at)]
    state.session_messages = kept[:]
    state.tokens_used = 0
    state.context_warned = False
    print_success(f"Rewound {count} turn(s). {len(kept) - 1} message(s) remaining (excluding system).")

def handle_remember(args: List[str]):
    from .memory import memory_manager
    if not args:
        print_error("Usage: /remember <key> <value>")
        return
    key = args[0]
    value = " ".join(args[1:])
    if not value:
        print_error("Usage: /remember <key> <value>")
        return
    msg = memory_manager.remember(key, value)
    print_success(msg)

def handle_forget(args: List[str]):
    from .memory import memory_manager
    if not args:
        print_error("Usage: /forget <key>")
        return
    msg = memory_manager.forget(args[0])
    _ui.console.print(f" [dim]{msg}[/dim]")

def handle_memories(args: List[str]):
    from .memory import memory_manager
    memories = memory_manager.list()
    if not memories:
        _ui.console.print("[dim]No memories saved yet. Use /remember <key> <value> to save one.[/dim]")
        return
    table = Table(title="Session Memory", show_header=True, header_style="bold cyan")
    table.add_column("Key", style="bold yellow")
    table.add_column("Value")
    for key, value in memories.items():
        table.add_row(key, value[:80])
    _ui.console.print(table)

def handle_edit(args: List[str], state):
    if not state.session_messages:
        print_error("No conversation to edit.")
        return
    try:
        idx = int(args[0]) if args else -1
    except ValueError:
        print_error("Usage: /edit <message_index>. Use /history to see indices.")
        return

    if idx < 0 or idx >= len(state.session_messages):
        print_error(f"Index {idx} out of range (0-{len(state.session_messages) - 1}). Use /history to see indices.")
        return

    msg = state.session_messages[idx]
    old_content = msg.get("content", "")
    if isinstance(old_content, list):
        old_content = " ".join(c.get("text", "") for c in old_content if isinstance(c, dict))

    try:
        from prompt_toolkit.shortcuts import input_dialog
        new_content = LoomDialog(
            title=f"Edit Message {idx} (role: {msg['role']})",
            text=_ui.get_dialog_text("Edit the message content:", "input"),
            default=old_content,
            dialog_type="input"
        ).run()
    except Exception:
        new_content = None

    if new_content is not None and new_content != old_content:
        msg["content"] = new_content
        state.session_messages = state.session_messages[:idx + 1]
        state.tokens_used = 0
        state.context_warned = False
        print_success(f"Message {idx} updated. Conversation truncated after edit.")
    elif new_content is not None:
        _ui.console.print("[dim]No changes made.[/dim]")

def handle_search(args: List[str], state):
    import re
    if not args:
        print_error("Usage: /search <term>")
        return
    term = " ".join(args)
    if not state.session_messages:
        _ui.console.print("[dim]No conversation to search.[/dim]")
        return

    results = []
    for i, msg in enumerate(state.session_messages):
        content = msg.get("content", "")
        if isinstance(content, list):
            content = " ".join(c.get("text", "") for c in content if isinstance(c, dict))
        if isinstance(content, str) and term.lower() in content.lower():
            results.append((i, msg["role"], content[:120]))

    if not results:
        _ui.console.print(f"[dim]No matches for '{term}'.[/dim]")
        return

    table = Table(title=f"Search: '{term}'", show_header=True, header_style="bold magenta")
    table.add_column("#", style="dim")
    table.add_column("Role", style="bold")
    table.add_column("Content")
    for idx, role, content in results:
        table.add_row(str(idx), role, content.replace("[", "\\["))
    _ui.console.print(table)

def handle_attach(args: List[str], state):
    if not args:
        print_error("Usage: /attach <file_path>")
        return
    path = " ".join(args)
    att = load_attachment(path)
    if att is None:
        print_error(f"File not found: {path}")
        return

    if att["type"] == "image":
        msg = {
            "role": "user",
            "content": [
                {"type": "text", "text": f"Attached image: {att['name']}"},
                {"type": "image_url", "image_url": {"url": f"data:{att['mime_type']};base64,{att['data']}"}}
            ]
        }
        state.session_messages.append(msg)
        print_success(f"Attached image: {att['name']} ({att['size'] // 1024}KB)")
    elif att["type"] == "text":
        msg = {
            "role": "user",
            "content": f"[Attached file: {att['name']}]\n```\n{att['content']}\n```"
        }
        state.session_messages.append(msg)
        line_count = att['content'].count('\n') + 1
        print_success(f"Attached file: {att['name']} ({line_count} lines)")
    else:
        msg = {
            "role": "user",
            "content": att.get("content", f"[File: {att['name']}]")
        }
        state.session_messages.append(msg)
        print_success(f"Attached: {att['name']}")

def handle_version(args: List[str]):
    _ui.console.print("[accent]LoomCLI[/accent] [white]0.1.0[/white]")
    _ui.console.print(f"[dim]Python based, {len(COMMANDS)} commands[/dim]")

def handle_theme(args: List[str]):
    from .ui import THEMES, apply_theme
    from prompt_toolkit.shortcuts import radiolist_dialog

    if args:
        name = args[0]
        if name in THEMES:
            apply_theme(name)
            config.theme = name
            config.save()
            # Force a clear and re-print of welcome screen to unify the background
            import sys, getpass
            from .ui import get_theme_bg, print_welcome_screen
            bg = get_theme_bg()
            r, g, b = int(bg[1:3], 16), int(bg[3:5], 16), int(bg[5:7], 16)
            sys.stdout.write(f"\033[48;2;{r};{g};{b}m\033[2J\033[H")
            sys.stdout.flush()
            print_welcome_screen(getpass.getuser(), config.model, config.provider)
            print_success(f"Theme set to: {name}")
        else:
            avail = ", ".join(THEMES.keys())
            print_error(f"Theme '{name}' not found. Available: {avail}")
        return

    active = config.theme
    choices = [(n, n.capitalize()) for n in THEMES]

    result = LoomDialog(
        title="Select Theme",
        text=_ui.get_dialog_text(f"Current: {active.capitalize()}", "radio"),
        values=choices,
        dialog_type="radio"
    ).run()

    if result:
        apply_theme(result)
        config.theme = result
        config.save()
        
        # Force a clear and re-print of welcome screen to unify the background
        import sys, getpass
        from .ui import get_theme_bg, print_welcome_screen
        bg = get_theme_bg()
        r, g, b = int(bg[1:3], 16), int(bg[3:5], 16), int(bg[5:7], 16)
        sys.stdout.write(f"\033[48;2;{r};{g};{b}m\033[2J\033[H")
        sys.stdout.flush()
        print_welcome_screen(getpass.getuser(), config.model, config.provider)
        print_success(f"Theme set to: {result}")

def handle_personality(args: List[str]):
    from .personalities import load_personalities, get_active_personality
    from prompt_toolkit.shortcuts import radiolist_dialog

    if args:
        name = args[0]
        personalities = load_personalities()
        if name in personalities:
            config.personality = name
            config.save()
            print_success(f"Personality set to: {name}")
        else:
            avail = ", ".join(personalities.keys())
            print_error(f"Personality '{name}' not found. Available: {avail}")
        return

    personalities = load_personalities()
    active = get_active_personality()
    choices = [(n, f"{n}: {p.description}") for n, p in personalities.items()]

    result = LoomDialog(
        title="Select Personality",
        text=_ui.get_dialog_text(f"Current: {active.name} ({active.description})", "radio"),
        values=choices,
        dialog_type="radio"
    ).run()

    if result:
        config.personality = result
        config.save()
        print_success(f"Personality set to: {result}")

COMMANDS = {
    "/help": handle_help,
    "/tools": handle_tools,
    "/provider": handle_provider,
    "/model": handle_model,
    "/personality": handle_personality,
    "/theme": handle_theme,
    "/config": handle_config,
    "/history": handle_history,
    "/save": handle_save,
    "/load": handle_load,
    "/tasks": handle_tasks,
    "/task-stop": handle_task_stop,
    "/rewind": handle_rewind,
    "/edit": handle_edit,
    "/search": handle_search,
    "/attach": handle_attach,
    "/version": handle_version,
    "/remember": handle_remember,
    "/forget": handle_forget,
    "/memories": handle_memories,
    "/clear": handle_clear,
    "/exit": handle_exit,
}

def get_command_metadata() -> Dict[str, str]:
    return {
        "/help": "Show help menu",
        "/tools": "List available agent tools",
        "/provider": "Select AI provider interactively",
        "/model": "Get or set the active model",
        "/personality": "Set the AI personality/style",
        "/theme": "Change the color theme",
        "/config": "Manage keys (Update, Delete, View)",
        "/history": "Show recent conversation messages",
        "/save [name]": "Save the current conversation",
        "/load": "Load a saved conversation",
        "/tasks": "List active and recent tasks",
        "/task-stop <id>": "Stop a running task",
        "/rewind [n]": "Remove the last N turns from the conversation",
        "/edit <idx>": "Edit a past message in the conversation",
        "/search <term>": "Search conversation history for a term",
        "/attach <file>": "Attach an image, text file, or PDF to the conversation",
        "/version": "Show version info",
        "/remember <key> <value>": "Save a memory for future sessions",
        "/forget <key>": "Delete a saved memory",
        "/memories": "List all saved memories",
        "/clear": "Clear the terminal screen",
        "/exit": "Exit the session",
    }

def execute_command(input_str: str, state) -> bool:
    from .skills import discover_skills
    from pathlib import Path
    parts = input_str.split()
    if not parts: return False
    command = parts[0]
    args = parts[1:]

    if command in COMMANDS:
        state.commands_run += 1
        # Check if the command expects state
        import inspect
        sig = inspect.signature(COMMANDS[command])
        if "state" in sig.parameters:
            COMMANDS[command](args, state)
        else:
            COMMANDS[command](args)
        return True

    # Check for skill-based commands
    skills = discover_skills()
    skill_name = command.lstrip("/")
    if skill_name in skills:
        from .ui import print_tool_call, print_tool_result
        skill = skills[skill_name]
        arg_str = " ".join(args)
        label = f"Skill({skill.name})"
        state.commands_run += 1
        _ui.print_tool_call(label, {})
        from .skills import run_skill
        result = run_skill(skill, arg_str)
        _ui.print_tool_result(result)
        return True

    return False
